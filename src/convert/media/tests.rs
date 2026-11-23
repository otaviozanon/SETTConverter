// Media codec and timestamp tests.

use super::*;

#[test]
fn timestamps() {
    assert_eq!(seconds("10"), Some(10.0));
    assert_eq!(seconds("01:30"), Some(90.0));
    assert_eq!(seconds("01:00:00"), Some(3600.0));
    assert_eq!(seconds("x"), None);
}

#[test]
fn avif_quality_maps_to_crf() {
    assert!(avif_crf(90) < avif_crf(30));
    assert!((10..=63).contains(&avif_crf(100)));
}

#[test]
fn cpu_arguments_cover_video_and_audio() {
    let mut command = Command::new("ffmpeg");
    codecs_cpu(&mut command, "mp4").unwrap();
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    assert!(args.contains(&"libx264".to_string()));
    assert!(args.contains(&"+faststart".to_string()));

    let mut command = Command::new("ffmpeg");
    codecs_cpu(&mut command, "mp3").unwrap();
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    assert!(args.contains(&"-vn".to_string()));
    assert!(args.contains(&"libmp3lame".to_string()));
}

#[test]
fn gif_arguments_use_the_palette_filter() {
    let mut command = Command::new("ffmpeg");
    codecs_cpu(&mut command, "gif").unwrap();
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    assert!(args.iter().any(|arg| arg.contains("palettegen")));
}
