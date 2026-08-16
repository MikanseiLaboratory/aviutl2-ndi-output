use std::time::Instant;

pub const TICKS_PER_SECOND: i64 = 10_000_000;
pub const MIN_VIDEO_DIM: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormatKind {
    Rgba,
    Rgbx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaError {
    TooSmall { width: u32, height: u32 },
    InvalidPitch { pitch: u32, width: u32 },
    BufferTooSmall { needed: usize, actual: usize },
    EmptyAudio,
    ChannelLengthMismatch,
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall { width, height } => {
                write!(
                    f,
                    "video {width}x{height} is smaller than the minimum {MIN_VIDEO_DIM}x{MIN_VIDEO_DIM}"
                )
            }
            Self::InvalidPitch { pitch, width } => {
                write!(f, "pitch {pitch} is smaller than width {width} * 4")
            }
            Self::BufferTooSmall { needed, actual } => {
                write!(f, "buffer has {actual} bytes, need at least {needed}")
            }
            Self::EmptyAudio => write!(f, "audio buffer is empty"),
            Self::ChannelLengthMismatch => {
                write!(f, "planar audio channels have different lengths")
            }
        }
    }
}

impl std::error::Error for MediaError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertedVideo {
    pub width: u32,
    pub height: u32,
    pub stride: i32,
    pub rgba: Vec<u8>,
    pub has_alpha: bool,
    pub pixel_format: PixelFormatKind,
}

/// Copy AviUtl2 scene RGBA8 (possibly padded pitch) into tightly packed RGBA8.
pub fn copy_rgba_rows(
    width: u32,
    height: u32,
    pitch: u32,
    src: &[u8],
) -> Result<ConvertedVideo, MediaError> {
    if width < MIN_VIDEO_DIM || height < MIN_VIDEO_DIM {
        return Err(MediaError::TooSmall { width, height });
    }
    let row_bytes = (width as usize).saturating_mul(4);
    let pitch_usize = pitch as usize;
    if pitch_usize < row_bytes {
        return Err(MediaError::InvalidPitch { pitch, width });
    }
    let needed = pitch_usize.saturating_mul(height as usize);
    if src.len() < needed {
        return Err(MediaError::BufferTooSmall {
            needed,
            actual: src.len(),
        });
    }

    let mut rgba = vec![0u8; row_bytes.saturating_mul(height as usize)];
    let mut has_alpha = false;
    for y in 0..height as usize {
        let src_off = y * pitch_usize;
        let dst_off = y * row_bytes;
        rgba[dst_off..dst_off + row_bytes].copy_from_slice(&src[src_off..src_off + row_bytes]);
        for x in 0..width as usize {
            if rgba[dst_off + x * 4 + 3] != 255 {
                has_alpha = true;
            }
        }
    }

    Ok(ConvertedVideo {
        width,
        height,
        stride: row_bytes as i32,
        rgba,
        has_alpha,
        pixel_format: PixelFormatKind::Rgba,
    })
}

/// Keep RGBA when alpha is enabled. Otherwise force A=255 and select RGBX.
pub fn apply_alpha_policy(mut video: ConvertedVideo, send_alpha: bool) -> ConvertedVideo {
    if send_alpha {
        video.pixel_format = PixelFormatKind::Rgba;
        return video;
    }
    for pixel in video.rgba.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    video.has_alpha = false;
    video.pixel_format = PixelFormatKind::Rgbx;
    video
}

/// Copy planar f32 channels into a channel-major `Vec<f32>` (left, then right).
pub fn planar_f32(planes: &[&[f32]]) -> Result<Vec<f32>, MediaError> {
    if planes.is_empty() || planes.iter().all(|p| p.is_empty()) {
        return Err(MediaError::EmptyAudio);
    }
    let samples = planes.iter().map(|p| p.len()).max().unwrap_or(0);
    if planes.iter().any(|p| !p.is_empty() && p.len() != samples) {
        return Err(MediaError::ChannelLengthMismatch);
    }
    let mut out = Vec::with_capacity(planes.len() * samples);
    for plane in planes {
        if plane.is_empty() {
            out.extend(std::iter::repeat_n(0.0, samples));
        } else {
            out.extend_from_slice(plane);
        }
    }
    Ok(out)
}

#[derive(Debug)]
pub struct SessionClock {
    start: Instant,
    last: i64,
}

impl SessionClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            last: -1,
        }
    }

    pub fn elapsed_ticks(&self) -> i64 {
        let nanos = self.start.elapsed().as_nanos();
        (nanos / 100) as i64
    }

    /// Monotonic 100 ns timecode derived from the session [`Instant`].
    pub fn next_monotonic(&mut self) -> i64 {
        let candidate = self.elapsed_ticks();
        let next = if candidate <= self.last {
            self.last.saturating_add(1)
        } else {
            candidate
        };
        self.last = next;
        next
    }

    pub fn last(&self) -> i64 {
        self.last
    }
}

impl Default for SessionClock {
    fn default() -> Self {
        Self::new()
    }
}

pub fn video_interval_ticks(fps_n: i32, fps_d: i32) -> i64 {
    let n = fps_n.max(1) as i64;
    let d = fps_d.max(1) as i64;
    (TICKS_PER_SECOND * d) / n
}

pub fn audio_interval_ticks(samples: i32, sample_rate: i32) -> i64 {
    let samples = samples.max(1) as i64;
    let rate = sample_rate.max(1) as i64;
    (TICKS_PER_SECOND * samples) / rate
}

pub fn samples_per_frame(sample_rate: u32, fps_n: i32, fps_d: i32) -> u64 {
    let rate = u64::from(sample_rate.max(1));
    let n = fps_n.max(1) as u64;
    let d = fps_d.max(1) as u64;
    (rate * d) / n
}

pub fn estimate_bake_bytes(
    width: u32,
    height: u32,
    frames: u32,
    sample_rate: u32,
    fps_n: i32,
    fps_d: i32,
) -> u64 {
    let frames = u64::from(frames.max(1));
    let video = frames * u64::from(width) * u64::from(height) * 4;
    let audio = frames * samples_per_frame(sample_rate, fps_n, fps_d) * 2 * 4;
    video.saturating_add(audio)
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.2} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.1} MiB", value / MIB)
    } else if value >= KIB {
        format!("{:.0} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playhead {
    Frame(u32),
    PastEnd,
}

/// Wall-clock playhead. Late callers jump to the current frame instead of catching up.
pub fn playhead_frame(elapsed_secs: f64, fps_n: i32, fps_d: i32, last_frame: u32) -> Playhead {
    let n = fps_n.max(1) as f64;
    let d = fps_d.max(1) as f64;
    let frame = (elapsed_secs * n / d).floor();
    if !frame.is_finite() || frame < 0.0 {
        return Playhead::Frame(0);
    }
    let frame = frame as u32;
    if frame > last_frame {
        Playhead::PastEnd
    } else {
        Playhead::Frame(frame)
    }
}

pub fn downscale_rgba(
    width: u32,
    height: u32,
    rgba: &[u8],
    max_width: u32,
) -> Option<(u32, u32, Vec<u8>)> {
    if width < MIN_VIDEO_DIM || height < MIN_VIDEO_DIM || max_width < 1 {
        return None;
    }
    let row = (width as usize).saturating_mul(4);
    if rgba.len() < row.saturating_mul(height as usize) {
        return None;
    }
    let dst_w = width.min(max_width).max(1);
    let dst_h = ((u64::from(height) * u64::from(dst_w)) / u64::from(width.max(1))).max(1) as u32;
    let mut out = vec![
        0u8;
        (dst_w as usize)
            .saturating_mul(dst_h as usize)
            .saturating_mul(4)
    ];
    for y in 0..dst_h as usize {
        let src_y = y * height as usize / dst_h as usize;
        for x in 0..dst_w as usize {
            let src_x = x * width as usize / dst_w as usize;
            let src = src_y * row + src_x * 4;
            let dst = (y * dst_w as usize + x) * 4;
            out[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    Some((dst_w, dst_h, out))
}
