// Media codec and timestamp tests.

use super::*;
use crate::config::Config;
use crate::convert::Options;

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
fn video_quality_maps_to_crf() {
    assert_eq!(video_crf(None, 23, 51), "23");
    assert_eq!(video_crf(Some(75), 23, 51), "23");
    assert_eq!(video_crf(Some(100), 23, 51), "10");
    assert_eq!(video_crf(Some(1), 23, 51), "51");
}

#[test]
fn cpu_arguments_cover_video_and_audio() {
    let options = Options::default();
    let cfg = Config::default();
    let mut command = Command::new("ffmpeg");
    codecs_cpu(&mut command, "mp4", &options, &cfg).unwrap();
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    assert!(args.contains(&"libx264".to_string()));
    assert!(args.contains(&"+faststart".to_string()));

    let mut command = Command::new("ffmpeg");
    codecs_cpu(&mut command, "mp3", &options, &cfg).unwrap();
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    assert!(args.contains(&"-vn".to_string()));
    assert!(args.contains(&"libmp3lame".to_string()));
    assert!(args.contains(&"-q:a".to_string()));
}

#[test]
fn audio_bitrate_reaches_lossy_codecs() {
    let options = Options::default();
    let cfg = Config {
        audio_bitrate: Some("96k".into()),
        ..Default::default()
    };
    for to in ["mp3", "ogg", "m4a", "opus", "mp4"] {
        let mut command = Command::new("ffmpeg");
        codecs_cpu(&mut command, to, &options, &cfg).unwrap();
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert!(
            args.windows(2).any(|pair| pair == ["-b:a", "96k"]),
            "{to} dropped the bitrate: {args:?}"
        );
    }
}

#[test]
fn video_quality_reaches_legacy_avi() {
    let cfg = Config::default();
    let mut command = Command::new("ffmpeg");
    codecs_cpu(&mut command, "avi", &Options::default(), &cfg).unwrap();
    let default: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    assert!(default.windows(2).any(|pair| pair == ["-q:v", "5"]));

    let mut command = Command::new("ffmpeg");
    codecs_cpu(
        &mut command,
        "avi",
        &Options {
            quality: Some(60),
            ..Default::default()
        },
        &cfg,
    )
    .unwrap();
    let set: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    assert!(set.windows(2).any(|pair| pair == ["-q:v", "14"]));
}

#[test]
fn gif_arguments_use_the_palette_filter() {
    let options = Options::default();
    let cfg = Config::default();
    let mut command = Command::new("ffmpeg");
    codecs_cpu(&mut command, "gif", &options, &cfg).unwrap();
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    assert!(args.iter().any(|arg| arg.contains("palettegen")));
    assert!(args.iter().any(|arg| arg.contains("fps=15")));
}
