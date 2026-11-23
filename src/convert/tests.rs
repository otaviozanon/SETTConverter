// Conversion pipeline tests.

use super::*;
use ::image::RgbaImage;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("settconv-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn options(to: &str) -> Options {
    Options {
        to: to.into(),
        ..Default::default()
    }
}

#[test]
fn image_pdf_merge_split_and_render() {
    let dir = temp("pipeline");
    let cfg = Config::default();
    let mut pdfs = Vec::new();
    for name in ["a.png", "b.png"] {
        let png = dir.join(name);
        RgbaImage::from_pixel(8, 8, ::image::Rgba([200, 30, 30, 255]))
            .save(&png)
            .unwrap();
        let pdf = run(&options("pdf"), std::slice::from_ref(&png), &cfg).unwrap();
        assert!(pdf[0].metadata().unwrap().len() > 0);
        pdfs.push(pdf[0].clone());
    }

    let merged = run(
        &Options {
            merge: true,
            ..options("pdf")
        },
        &pdfs,
        &cfg,
    )
    .unwrap();
    let parts = run(
        &Options {
            split_every: Some(1),
            ..options("pdf")
        },
        &merged,
        &cfg,
    )
    .unwrap();
    assert_eq!(parts.len(), 2);

    let pages = run(
        &Options {
            dpi: Some(72),
            ..options("png")
        },
        &parts[0..1],
        &cfg,
    )
    .unwrap();
    assert_eq!(pages.len(), 1);
    assert!(pages[0].metadata().unwrap().len() > 0);
}

#[test]
fn svg_to_png() {
    let dir = temp("svg");
    let svg = dir.join("logo.svg");
    std::fs::write(
            &svg,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#ff6600"/></svg>"##,
        )
        .unwrap();
    let png = run(
        &options("png"),
        std::slice::from_ref(&svg),
        &Config::default(),
    )
    .unwrap();
    assert_eq!(png.len(), 1);
    assert!(png[0].metadata().unwrap().len() > 0);
}

#[test]
fn batch_keeps_every_output() {
    let dir = temp("batch");
    let cfg = Config::default();
    let mut inputs = Vec::new();
    for index in 0..4 {
        let png = dir.join(format!("p{index}.png"));
        RgbaImage::from_pixel(4, 4, ::image::Rgba([index, 0, 0, 255]))
            .save(&png)
            .unwrap();
        inputs.push(png);
    }
    let out = run(&options("jpg"), &inputs, &cfg).unwrap();
    assert_eq!(out.len(), 4);
    assert!(out.iter().all(|path| path.metadata().unwrap().len() > 0));
}

#[test]
fn folder_to_pdf_and_long_image() {
    let dir = temp("folder");
    for name in ["a.png", "b.png"] {
        RgbaImage::from_pixel(16, 8, ::image::Rgba([0, 100, 200, 255]))
            .save(dir.join(name))
            .unwrap();
    }
    let out = run(
        &options("pdf"),
        std::slice::from_ref(&dir),
        &Config::default(),
    )
    .unwrap();
    assert_eq!(out.len(), 1);
    let long = run(
        &Options {
            long: true,
            dpi: Some(36),
            ..options("png")
        },
        &out,
        &Config::default(),
    )
    .unwrap();
    assert!(long[0].metadata().unwrap().len() > 0);
}

#[test]
fn folder_batch_converts_images() {
    let dir = temp("folder-batch");
    for name in ["a.png", "b.png"] {
        RgbaImage::from_pixel(4, 4, ::image::Rgba([10, 20, 30, 255]))
            .save(dir.join(name))
            .unwrap();
    }
    let out = run(
        &options("jpg"),
        std::slice::from_ref(&dir),
        &Config::default(),
    )
    .unwrap();
    assert_eq!(out.len(), 2);
}
