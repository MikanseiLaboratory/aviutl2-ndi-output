use aviutl2_ndi_live_output::media::{
    PixelFormatKind, SessionClock, TICKS_PER_SECOND, apply_alpha_policy, audio_interval_ticks,
    copy_rgba_rows, planar_f32, video_interval_ticks,
};

#[test]
fn padded_pitch_rgba_keeps_channel_order_and_alpha() {
    let width = 16u32;
    let height = 16u32;
    let pitch = width * 4 + 8;
    let mut src = vec![0u8; (pitch * height) as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let i = y * pitch as usize + x * 4;
            src[i] = 10;
            src[i + 1] = 20;
            src[i + 2] = 30;
            src[i + 3] = if x == 0 && y == 0 { 128 } else { 255 };
        }
    }

    let converted = copy_rgba_rows(width, height, pitch, &src).expect("convert");
    assert_eq!(converted.width, 16);
    assert_eq!(converted.height, 16);
    assert_eq!(converted.stride, 16 * 4);
    assert!(converted.has_alpha);
    assert_eq!(converted.pixel_format, PixelFormatKind::Rgba);
    assert_eq!(&converted.rgba[0..4], &[10, 20, 30, 128]);
    assert_eq!(&converted.rgba[4..8], &[10, 20, 30, 255]);
}

#[test]
fn opaque_rgba_does_not_set_alpha() {
    let width = 16u32;
    let height = 16u32;
    let pitch = width * 4;
    let src = vec![255u8; (pitch * height) as usize];
    let converted = copy_rgba_rows(width, height, pitch, &src).expect("convert");
    assert!(!converted.has_alpha);
}

#[test]
fn alpha_disabled_normalizes_to_rgbx() {
    let width = 16u32;
    let height = 16u32;
    let pitch = width * 4;
    let mut src = vec![255u8; (pitch * height) as usize];
    src[3] = 64;
    let converted = apply_alpha_policy(
        copy_rgba_rows(width, height, pitch, &src).expect("convert"),
        false,
    );
    assert!(!converted.has_alpha);
    assert_eq!(converted.pixel_format, PixelFormatKind::Rgbx);
    assert_eq!(converted.rgba[3], 255);
}

#[test]
fn alpha_enabled_keeps_rgba() {
    let width = 16u32;
    let height = 16u32;
    let pitch = width * 4;
    let mut src = vec![255u8; (pitch * height) as usize];
    src[3] = 64;
    let converted = apply_alpha_policy(
        copy_rgba_rows(width, height, pitch, &src).expect("convert"),
        true,
    );
    assert!(converted.has_alpha);
    assert_eq!(converted.pixel_format, PixelFormatKind::Rgba);
    assert_eq!(converted.rgba[3], 64);
}

#[test]
fn rejects_video_smaller_than_minimum() {
    let src = vec![0u8; 4];
    assert!(copy_rgba_rows(1, 1, 4, &src).is_err());
}

#[test]
fn rejects_invalid_pitch_and_short_buffer() {
    let src = vec![0u8; 8];
    assert!(copy_rgba_rows(4, 2, 8, &src).is_err());
    let src = vec![0u8; 16];
    assert!(copy_rgba_rows(4, 2, 16, &src).is_err());
}

#[test]
fn planar_f32_stereo_is_channel_major() {
    let left = [1.0f32, -1.0];
    let right = [0.5f32, 0.25];
    let samples = planar_f32(&[&left, &right]).expect("audio");
    assert_eq!(samples, vec![1.0, -1.0, 0.5, 0.25]);
}

#[test]
fn empty_right_channel_is_zero_filled() {
    let left = [0.25f32, 0.5];
    let samples = planar_f32(&[&left, &[]]).expect("audio");
    assert_eq!(samples, vec![0.25, 0.5, 0.0, 0.0]);
}

#[test]
fn timestamps_are_100ns_and_monotonic_across_loop_and_seek() {
    let mut clock = SessionClock::new();
    let first = clock.next_monotonic();
    assert!(first >= 0);
    let second = clock.next_monotonic();
    assert!(second > first);

    let mut samples = vec![first, second];
    for _ in 0..8 {
        samples.push(clock.next_monotonic());
    }
    for pair in samples.windows(2) {
        assert!(pair[1] > pair[0], "timestamp went backwards: {pair:?}");
    }

    assert_eq!(video_interval_ticks(60, 1), TICKS_PER_SECOND / 60);
    assert_eq!(audio_interval_ticks(480, 48_000), TICKS_PER_SECOND / 100);
}

#[test]
fn playhead_skips_ahead_instead_of_queueing() {
    use aviutl2_ndi_live_output::media::{Playhead, playhead_frame};
    assert_eq!(playhead_frame(0.0, 30, 1, 299), Playhead::Frame(0));
    assert_eq!(playhead_frame(1.0, 30, 1, 299), Playhead::Frame(30));
    assert_eq!(playhead_frame(20.0, 30, 1, 299), Playhead::PastEnd);
}
