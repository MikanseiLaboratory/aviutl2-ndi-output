//! NDI® sender/receiver loopback for RGBA video and stereo audio.

use std::thread;
use std::time::{Duration, Instant};

use aviutl2_ndi_live_output::media::{apply_alpha_policy, copy_rgba_rows, planar_f32};
use aviutl2_ndi_live_output::ndi::prepare_ndi_runtime;
use grafton_ndi::{
    AudioFrame, AudioLayout, Finder, FinderOptions, NDI, PixelFormat, Receiver, ReceiverBandwidth,
    ReceiverColorFormat, ReceiverOptions, Sender, SenderOptions, VideoFrame,
};

fn require_ndi() -> Option<NDI> {
    if let Err(e) = prepare_ndi_runtime() {
        eprintln!("skipping NDI® loopback: {e}");
        return None;
    }
    match NDI::new() {
        Ok(ndi) => Some(ndi),
        Err(e) => {
            eprintln!("skipping NDI® loopback: runtime init failed: {e}");
            None
        }
    }
}

fn find_source(ndi: &NDI, needle: &str) -> grafton_ndi::Source {
    let finder = Finder::new(
        ndi,
        &FinderOptions::builder().show_local_sources(true).build(),
    )
    .expect("finder");
    for _ in 0..40 {
        let _ = finder.wait_for_sources(Duration::from_millis(250));
        if let Ok(sources) = finder.current_sources()
            && let Some(source) = sources.into_iter().find(|s| s.name.contains(needle))
        {
            return source;
        }
    }
    panic!("did not discover NDI® source containing {needle}");
}

fn wait_connected(sender: &Sender, connected: bool) {
    for _ in 0..80 {
        let count = sender.connection_count(Duration::ZERO).unwrap_or(0);
        if connected && count > 0 || !connected && count == 0 {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn solid_rgba(size: u32, r: u8, g: u8, b: u8, a: u8, timecode: i64) -> VideoFrame {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[r, g, b, a]);
    }
    let converted = apply_alpha_policy(
        copy_rgba_rows(size, size, size * 4, &rgba).expect("convert"),
        a != 255,
    );
    let format = match converted.pixel_format {
        aviutl2_ndi_live_output::media::PixelFormatKind::Rgba => PixelFormat::RGBA,
        aviutl2_ndi_live_output::media::PixelFormatKind::Rgbx => PixelFormat::RGBX,
    };
    let mut frame = VideoFrame::builder()
        .resolution(converted.width as i32, converted.height as i32)
        .pixel_format(format)
        .frame_rate(60, 1)
        .timecode(timecode)
        .build()
        .expect("video frame");
    frame.replace_data(converted.rgba).expect("replace");
    frame
}

fn stereo_audio(timecode: i64) -> AudioFrame {
    let left = vec![0.25f32; 480];
    let right = vec![-0.5f32; 480];
    let pcm = planar_f32(&[&left, &right]).expect("audio");
    AudioFrame::builder()
        .sample_rate(48_000)
        .channels(2)
        .samples(480)
        .layout(AudioLayout::Planar)
        .timecode(timecode)
        .data(pcm)
        .build()
        .expect("audio frame")
}

fn recv_video_with_timecode(
    sender: &Sender,
    receiver: &Receiver,
    frame: &VideoFrame,
    timecode: i64,
) -> VideoFrame {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        sender.send_video(frame);
        if let Ok(Some(got)) = receiver.video().try_capture(Duration::from_millis(50))
            && got.timecode() == timecode
        {
            return got;
        }
    }
    panic!("did not receive video with timecode {timecode}");
}

fn recv_audio_with_timecode(
    sender: &Sender,
    receiver: &Receiver,
    frame: &AudioFrame,
    timecode: i64,
) -> AudioFrame {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        sender.send_audio(frame);
        if let Ok(Some(got)) = receiver.audio().try_capture(Duration::from_millis(50))
            && got.timecode() == timecode
        {
            return got;
        }
    }
    panic!("did not receive audio with timecode {timecode}");
}

#[test]
fn rgba_stereo_loopback_timecode_tally_and_reconnect() {
    let Some(ndi) = require_ndi() else {
        return;
    };
    let name = format!("aviutl2-ndi-loopback-{}", std::process::id());
    let sender = Sender::new(
        &ndi,
        &SenderOptions::builder(&name)
            .clock_video(false)
            .clock_audio(true)
            .build(),
    )
    .expect("sender");

    let source = find_source(&ndi, &name);
    let receiver = Receiver::new(
        &ndi,
        &ReceiverOptions::builder(source)
            .color(ReceiverColorFormat::RGBX_RGBA)
            .bandwidth(ReceiverBandwidth::Highest)
            .name("aviutl2-ndi-loopback-rx")
            .build(),
    )
    .expect("receiver");
    wait_connected(&sender, true);
    assert!(sender.connection_count(Duration::ZERO).unwrap_or(0) > 0);
    let _ = sender.tally(Duration::from_millis(100));

    let video = solid_rgba(32, 10, 20, 30, 255, 1_000_000);
    let got_video = recv_video_with_timecode(&sender, &receiver, &video, 1_000_000);
    assert_eq!(got_video.width(), 32);
    assert_eq!(got_video.height(), 32);
    assert_eq!(got_video.frame_rate_n(), 60);
    let pixels = got_video.data();
    assert!(pixels.len() >= 4);
    assert_eq!(&pixels[0..3], &[10, 20, 30]);

    let audio = stereo_audio(1_000_100);
    let got_audio = recv_audio_with_timecode(&sender, &receiver, &audio, 1_000_100);
    assert_eq!(got_audio.num_channels(), 2);
    assert_eq!(got_audio.sample_rate(), 48_000);

    drop(receiver);
    wait_connected(&sender, false);

    let source = find_source(&ndi, &name);
    let receiver2 = Receiver::new(
        &ndi,
        &ReceiverOptions::builder(source)
            .color(ReceiverColorFormat::RGBX_RGBA)
            .bandwidth(ReceiverBandwidth::Highest)
            .name("aviutl2-ndi-loopback-rx2")
            .build(),
    )
    .expect("reconnect");
    wait_connected(&sender, true);
    let video = recv_video_with_timecode(
        &sender,
        &receiver2,
        &solid_rgba(32, 1, 2, 3, 255, 2_000_000),
        2_000_000,
    );
    assert_eq!(video.width(), 32);
}

#[test]
fn mid_connect_receives_later_frames() {
    let Some(ndi) = require_ndi() else {
        return;
    };
    let name = format!("aviutl2-ndi-mid-connect-{}", std::process::id());
    let sender = Sender::new(
        &ndi,
        &SenderOptions::builder(&name)
            .clock_video(false)
            .clock_audio(true)
            .build(),
    )
    .expect("sender");

    for _ in 0..4 {
        sender.send_video(&solid_rgba(16, 40, 50, 60, 255, 100));
        thread::sleep(Duration::from_millis(10));
    }

    let source = find_source(&ndi, &name);
    let receiver = Receiver::new(
        &ndi,
        &ReceiverOptions::builder(source)
            .color(ReceiverColorFormat::RGBX_RGBA)
            .bandwidth(ReceiverBandwidth::Highest)
            .build(),
    )
    .expect("late connect");
    wait_connected(&sender, true);

    let video = recv_video_with_timecode(
        &sender,
        &receiver,
        &solid_rgba(16, 40, 50, 60, 255, 9_000_000),
        9_000_000,
    );
    assert_eq!(video.timecode(), 9_000_000);
}
