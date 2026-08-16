use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize};

use aviutl2_ndi_live_output::media::{Playhead, downscale_rgba, playhead_frame};
use aviutl2_ndi_live_output::ndi::{LatestSlot, drop_channel};

#[test]
fn video_latest_wins_and_audio_queue_overflow() {
    let video = LatestSlot::new();
    video.push(1u32);
    video.push(2u32);
    video.push(3u32);
    assert_eq!(video.drops(), 2);
    assert_eq!(video.take(), Some(3));

    let drops = Arc::new(AtomicU64::new(0));
    let queued = Arc::new(AtomicUsize::new(0));
    let (audio_tx, audio_rx) = drop_channel::<&'static str>(1, Arc::clone(&drops), queued);
    assert!(audio_tx.try_send("a1"));
    assert!(!audio_tx.try_send("a2"));
    assert_eq!(drops.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(audio_rx.try_recv().ok(), Some("a1"));
    assert_eq!(audio_tx.drops(), 1);
}

#[test]
fn playhead_jumps_to_current_frame_and_past_end() {
    assert_eq!(playhead_frame(0.0, 30, 1, 99), Playhead::Frame(0));
    assert_eq!(playhead_frame(1.0, 30, 1, 99), Playhead::Frame(30));
    assert_eq!(playhead_frame(10.0, 30, 1, 99), Playhead::PastEnd);
    assert_eq!(playhead_frame(-1.0, 30, 1, 99), Playhead::Frame(0));
}

#[test]
fn preview_downscale_does_not_keep_full_width() {
    let width = 640u32;
    let height = 360u32;
    let src = vec![128u8; (width * height * 4) as usize];
    let (w, h, out) = downscale_rgba(width, height, &src, 320).expect("downscale");
    assert_eq!(w, 320);
    assert_eq!(h, 180);
    assert_eq!(out.len(), 320 * 180 * 4);
}
