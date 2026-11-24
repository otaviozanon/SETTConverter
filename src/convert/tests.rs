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

#[test]
fn png_keeps_alpha_at_every_compression_level() {
    let dir = temp("png-compression");
    let source = dir.join("alpha.png");
    RgbaImage::from_fn(32, 32, |x, y| {
        ::image::Rgba([(x * 8) as u8, (y * 8) as u8, 0, 128])
    })
    .save(&source)
    .unwrap();

    for level in [0, 9] {
        let out = run(
            &options("png"),
            std::slice::from_ref(&source),
            &Config {
                png_compression: Some(level),
                ..Default::default()
            },
        )
        .unwrap();
        let decoded = ::image::open(&out[0]).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (32, 32));
        assert_eq!(decoded.get_pixel(1, 2)[3], 128);
    }
}

#[test]
fn webp_quality_changes_the_output() {
    let dir = temp("webp-quality");
    let source = dir.join("noise.png");
    RgbaImage::from_fn(64, 64, |x, y| {
        ::image::Rgba([(x * 7) as u8, (y * 13) as u8, ((x + y) * 17) as u8, 255])
    })
    .save(&source)
    .unwrap();

    let low = run(
        &options("webp"),
        std::slice::from_ref(&source),
        &Config {
            webp_quality: Some(10),
            ..Default::default()
        },
    )
    .unwrap();
    let high = run(
        &options("webp"),
        std::slice::from_ref(&source),
        &Config {
            webp_quality: Some(95),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        low[0].metadata().unwrap().len() < high[0].metadata().unwrap().len(),
        "low quality should be smaller"
    );
}

#[test]
fn pdf_jpeg_is_opt_in() {
    let dir = temp("pdf-jpeg");
    let source = dir.join("noise.png");
    RgbaImage::from_fn(64, 64, |x, y| {
        ::image::Rgba([(x * 11) as u8, (y * 5) as u8, ((x * y) % 255) as u8, 255])
    })
    .save(&source)
    .unwrap();

    let lossless = run(
        &options("pdf"),
        std::slice::from_ref(&source),
        &Config::default(),
    )
    .unwrap();
    let jpeg = run(
        &options("pdf"),
        std::slice::from_ref(&source),
        &Config {
            pdf_jpeg: Some(true),
            ..Default::default()
        },
    )
    .unwrap();

    let bytes = std::fs::read(&jpeg[0]).unwrap();
    assert!(bytes.windows(9).any(|window| window == b"DCTDecode"));
    assert!(jpeg[0].metadata().unwrap().len() < lossless[0].metadata().unwrap().len());
}
