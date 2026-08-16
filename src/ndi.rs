use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use grafton_ndi::{AudioFrame, AudioLayout, NDI, PixelFormat, Sender, SenderOptions, VideoFrame};
use windows_sys::Win32::Foundation::{GetLastError, HMODULE, MAX_PATH};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW, SetDllDirectoryW,
};

use crate::config::PluginConfig;
use crate::controller::SharedState;
use crate::media::PixelFormatKind;

const NDI_DLL: &str = "Processing.NDI.Lib.x64.dll";

#[derive(Debug, Clone)]
pub struct VideoJob {
    pub width: u32,
    pub height: u32,
    pub stride: i32,
    pub rgba: Vec<u8>,
    pub pixel_format: PixelFormatKind,
    pub timecode: i64,
    pub fps_n: i32,
    pub fps_d: i32,
}

#[derive(Debug, Clone)]
pub struct AudioJob {
    pub data: Vec<f32>,
    pub timecode: i64,
    pub sample_rate: i32,
    pub channels: i32,
    pub samples_per_channel: i32,
}

#[derive(Debug)]
pub struct LatestSlot<T> {
    slot: Mutex<Option<T>>,
    drops: AtomicU64,
}

impl<T> LatestSlot<T> {
    pub fn new() -> Self {
        Self {
            slot: Mutex::new(None),
            drops: AtomicU64::new(0),
        }
    }

    pub fn push(&self, item: T) {
        let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        if slot.replace(item).is_some() {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn take(&self) -> Option<T> {
        self.slot.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    pub fn clear(&self) {
        let _ = self.take();
    }

    pub fn len(&self) -> usize {
        usize::from(
            self.slot
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_some(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn drops(&self) -> u64 {
        self.drops.load(Ordering::Relaxed)
    }
}

impl<T> Default for LatestSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DropSender<T> {
    tx: SyncSender<T>,
    drops: Arc<AtomicU64>,
    queued: Arc<AtomicUsize>,
}

impl<T: Send> DropSender<T> {
    pub fn try_send(&self, item: T) -> bool {
        match self.tx.try_send(item) {
            Ok(()) => {
                self.queued.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.drops.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub fn drops(&self) -> u64 {
        self.drops.load(Ordering::Relaxed)
    }
}

pub fn drop_channel<T: Send>(
    depth: usize,
    drops: Arc<AtomicU64>,
    queued: Arc<AtomicUsize>,
) -> (DropSender<T>, Receiver<T>) {
    drops.store(0, Ordering::Relaxed);
    queued.store(0, Ordering::Relaxed);
    let depth = depth.max(1);
    let (tx, rx) = sync_channel(depth);
    (
        DropSender {
            tx,
            drops,
            queued: Arc::clone(&queued),
        },
        rx,
    )
}

pub struct SendSession {
    stop: Arc<AtomicBool>,
    joins: Vec<JoinHandle<()>>,
}

impl SendSession {
    pub fn start(
        shared: Arc<SharedState>,
        config: PluginConfig,
        audio_rx: Receiver<AudioJob>,
    ) -> Result<Self, String> {
        prepare_ndi_runtime()?;
        let ndi = NDI::new().map_err(|e| format!("NDI® runtime init failed: {e}"))?;
        let sdk_version = NDI::version().unwrap_or_else(|_| "unknown".into());
        {
            let mut status = shared.status.lock().unwrap_or_else(|e| e.into_inner());
            status.sdk_version = sdk_version;
            status.last_error = None;
        }

        let mut builder = SenderOptions::builder(config.source_name.clone())
            .clock_video(config.send_video)
            .clock_audio(config.send_audio);
        if let Some(groups) = config.groups_opt() {
            builder = builder.groups(groups);
        }
        let options = builder.build();
        let sender =
            Sender::new(&ndi, &options).map_err(|e| format!("NDI® sender create failed: {e}"))?;
        let sender = Arc::new(sender);
        drop(ndi);

        let stop = Arc::new(AtomicBool::new(false));
        let mut joins = Vec::new();

        if config.send_video {
            let video_stop = Arc::clone(&stop);
            let video_shared = Arc::clone(&shared);
            let video_sender = Arc::clone(&sender);
            joins.push(
                thread::Builder::new()
                    .name("ndi-video".into())
                    .spawn(move || {
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            video_loop(video_sender, video_shared, video_stop);
                        }));
                    })
                    .map_err(|e| format!("failed to spawn NDI® video worker: {e}"))?,
            );
        }

        if config.send_audio {
            let audio_stop = Arc::clone(&stop);
            let audio_shared = Arc::clone(&shared);
            let audio_sender = Arc::clone(&sender);
            let queued = Arc::clone(&shared.audio_queued);
            joins.push(
                thread::Builder::new()
                    .name("ndi-audio".into())
                    .spawn(move || {
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            audio_loop(audio_sender, audio_shared, audio_rx, queued, audio_stop);
                        }));
                    })
                    .map_err(|e| format!("failed to spawn NDI® audio worker: {e}"))?,
            );
        }

        let monitor_stop = Arc::clone(&stop);
        let monitor_shared = Arc::clone(&shared);
        let monitor_sender = Arc::clone(&sender);
        let send_video = config.send_video;
        let send_audio = config.send_audio;
        joins.push(
            thread::Builder::new()
                .name("ndi-monitor".into())
                .spawn(move || {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        monitor_loop(
                            monitor_sender,
                            monitor_shared,
                            send_video,
                            send_audio,
                            monitor_stop,
                        );
                    }));
                })
                .map_err(|e| format!("failed to spawn NDI® monitor: {e}"))?,
        );

        Ok(Self { stop, joins })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for join in self.joins.drain(..) {
            let _ = join.join();
        }
    }
}

impl Drop for SendSession {
    fn drop(&mut self) {
        self.stop();
    }
}

fn video_loop(sender: Arc<Sender>, shared: Arc<SharedState>, stop: Arc<AtomicBool>) {
    let mut last_sample = Instant::now();
    let mut frames = 0u64;
    while !stop.load(Ordering::Acquire) {
        if let Some(job) = shared.video_slot.take() {
            match video_frame(job) {
                Ok(frame) => {
                    sender.send_video(&frame);
                    frames += 1;
                }
                Err(e) => shared.set_error(format!("video frame: {e}")),
            }
        } else {
            thread::sleep(Duration::from_millis(1));
        }

        let elapsed = last_sample.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            let fps = if elapsed > 0.0 {
                frames as f32 / elapsed
            } else {
                0.0
            };
            shared
                .status
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .send_fps = fps;
            last_sample = Instant::now();
            frames = 0;
        }
    }
}

fn audio_loop(
    sender: Arc<Sender>,
    shared: Arc<SharedState>,
    rx: Receiver<AudioJob>,
    queued: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Acquire) {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(job) => {
                queued.fetch_sub(1, Ordering::Relaxed);
                match audio_frame(job) {
                    Ok(frame) => sender.send_audio(&frame),
                    Err(e) => shared.set_error(format!("audio frame: {e}")),
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

struct MonitorArgs {
    sender: Arc<Sender>,
    shared: Arc<SharedState>,
    send_video: bool,
    send_audio: bool,
    stop: Arc<AtomicBool>,
}

fn monitor_loop(
    sender: Arc<Sender>,
    shared: Arc<SharedState>,
    send_video: bool,
    send_audio: bool,
    stop: Arc<AtomicBool>,
) {
    let args = MonitorArgs {
        sender,
        shared,
        send_video,
        send_audio,
        stop,
    };
    while !args.stop.load(Ordering::Acquire) {
        let connections = args.sender.connection_count(Duration::ZERO).unwrap_or(0);
        let tally = args.sender.tally(Duration::ZERO).ok().flatten();
        let connected = connections > 0;
        {
            let mut status = args.shared.status.lock().unwrap_or_else(|e| e.into_inner());
            status.connections = connections;
            status.video_subscribed = args.send_video && connected;
            status.audio_subscribed = args.send_audio && connected;
            status.tally_program = tally.as_ref().is_some_and(|t| t.on_program);
            status.tally_preview = tally.as_ref().is_some_and(|t| t.on_preview);
            status.video_drops = args.shared.video_slot.drops();
            status.audio_drops = args.shared.audio_drops.load(Ordering::Relaxed);
            status.queue_depth = args.shared.audio_queued.load(Ordering::Relaxed);
            status.video_queued = args.shared.video_slot.len();
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn video_frame(job: VideoJob) -> Result<VideoFrame, String> {
    let pixel_format = match job.pixel_format {
        PixelFormatKind::Rgba => PixelFormat::RGBA,
        PixelFormatKind::Rgbx => PixelFormat::RGBX,
    };
    let mut frame = VideoFrame::builder()
        .resolution(job.width as i32, job.height as i32)
        .pixel_format(pixel_format)
        .frame_rate(job.fps_n, job.fps_d)
        .timecode(job.timecode)
        .build()
        .map_err(|e| e.to_string())?;
    frame.replace_data(job.rgba).map_err(|e| e.to_string())?;
    let _ = job.stride;
    Ok(frame)
}

fn audio_frame(job: AudioJob) -> Result<AudioFrame, String> {
    AudioFrame::builder()
        .sample_rate(job.sample_rate)
        .channels(job.channels)
        .samples(job.samples_per_channel)
        .layout(AudioLayout::Planar)
        .timecode(job.timecode)
        .data(job.data)
        .build()
        .map_err(|e| e.to_string())
}

pub fn prepare_ndi_runtime() -> Result<PathBuf, String> {
    let dir = find_ndi_runtime_dir()?;
    set_dll_directory(&dir)?;
    let dll = dir.join(NDI_DLL);
    if !dll.is_file() {
        return Err(format!(
            "NDI® runtime DLL not found at {}. Place {NDI_DLL} next to the plugin.",
            dll.display()
        ));
    }
    Ok(dll)
}

fn find_ndi_runtime_dir() -> Result<PathBuf, String> {
    let mut dirs = Vec::new();
    if let Ok(plugin_dir) = current_module_dir() {
        dirs.push(plugin_dir.clone());
        dirs.push(plugin_dir.join("aviutl2_ndi_live_output"));
    }
    if let Ok(sdk) = std::env::var("NDI_SDK_DIR") {
        dirs.push(PathBuf::from(sdk).join("Bin/x64"));
    }
    if let Ok(runtime) = std::env::var("NDI_RUNTIME_DIR_V6") {
        dirs.push(PathBuf::from(runtime));
    }
    dirs.push(PathBuf::from(r"C:\Program Files\NDI\NDI 6 SDK\Bin\x64"));
    dirs.push(PathBuf::from(r"C:\Program Files\NDI\NDI 6 Runtime\v6"));

    dirs.into_iter()
        .find(|dir| dir.join(NDI_DLL).is_file())
        .ok_or_else(|| {
            format!("NDI® runtime ({NDI_DLL}) was not found next to the plugin or in NDI_SDK_DIR")
        })
}

fn current_module_dir() -> Result<PathBuf, String> {
    unsafe {
        let mut module: HMODULE = std::ptr::null_mut();
        let ok = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            current_module_dir as *const () as *const u16,
            &mut module,
        );
        if ok == 0 || module.is_null() {
            return Err(format!("GetModuleHandleExW failed ({})", GetLastError()));
        }
        let mut buf = [0u16; MAX_PATH as usize + 1];
        let len = GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32);
        if len == 0 {
            return Err(format!("GetModuleFileNameW failed ({})", GetLastError()));
        }
        let path = PathBuf::from(OsString::from_wide(&buf[..len as usize]));
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "plugin path has no parent directory".into())
    }
}

fn set_dll_directory(dir: &Path) -> Result<(), String> {
    let mut wide: Vec<u16> = dir.as_os_str().encode_wide().collect();
    wide.push(0);
    let ok = unsafe { SetDllDirectoryW(wide.as_ptr()) };
    if ok == 0 {
        Err(format!(
            "SetDllDirectoryW({}) failed ({})",
            dir.display(),
            unsafe { GetLastError() }
        ))
    } else {
        Ok(())
    }
}
