// Command line parsing, kept separate from the Windows shell.

use anyhow::{Result, bail};
use std::path::PathBuf;

use crate::convert::Options;
use crate::formats;
use crate::lang::t;

pub enum Command {
    MenuInstall,
    MenuUninstall,
    Extract {
        options: Options,
        inputs: Vec<PathBuf>,
    },
    Convert {
        options: Options,
        inputs: Vec<PathBuf>,
    },
    Formats,
    Version,
    Help,
}

pub fn parse(args: &[String]) -> Result<Command> {
    let Some(command) = args.first() else {
        return Ok(Command::Help);
    };
    let rest = &args[1..];
    match command.as_str() {
        "menu" => match rest.first().map(String::as_str) {
            Some("install") => Ok(Command::MenuInstall),
            Some("uninstall") => Ok(Command::MenuUninstall),
            _ => bail!(
                "{}",
                t(
                    "uso: SETTConverter menu install|uninstall",
                    "usage: SETTConverter menu install|uninstall"
                )
            ),
        },
        "extract" => extract(rest),
        "convert" => convert(rest),
        "formats" => Ok(Command::Formats),
        "version" | "-V" | "--version" => Ok(Command::Version),
        "-h" | "--help" | "help" => Ok(Command::Help),
        other => bail!("{} {other}", t("comando desconhecido:", "unknown command:")),
    }
}

fn extract(rest: &[String]) -> Result<Command> {
    let mut options = Options {
        to: "extract".into(),
        ..Default::default()
    };
    let mut inputs = Vec::new();
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--out" => {
                index += 1;
                let value = rest.get(index).ok_or_else(|| {
                    anyhow::anyhow!("{}", t("--out sem valor", "--out needs a value"))
                })?;
                options.out = Some(PathBuf::from(value));
            }
            "--force" => options.force = true,
            other if other.starts_with('-') => {
                bail!("{} {other}", t("opcao desconhecida:", "unknown option:"))
            }
            other => inputs.push(PathBuf::from(other)),
        }
        index += 1;
    }
    if inputs.is_empty() {
        bail!("{}", t("nenhum arquivo informado", "no input files"));
    }
    Ok(Command::Extract { options, inputs })
}

fn convert(rest: &[String]) -> Result<Command> {
    let mut options = Options::default();
    let mut inputs = Vec::new();
    let mut index = 0;

    while index < rest.len() {
        let arg = rest[index].clone();
        let value = |index: &mut usize| -> Result<String> {
            *index += 1;
            rest.get(*index)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("{arg} {}", t("sem valor", "needs a value")))
        };
        match arg.as_str() {
            "--to" => options.to = value(&mut index)?.to_lowercase(),
            "--out" => options.out = Some(PathBuf::from(value(&mut index)?)),
            "--quality" => options.quality = Some(value(&mut index)?.parse()?),
            "--split-every" => options.split_every = Some(value(&mut index)?.parse()?),
            "--width" => options.width = Some(value(&mut index)?.parse()?),
            "--dpi" => options.dpi = Some(value(&mut index)?.parse()?),
            "--size" => options.size = Some(parse_size(&value(&mut index)?)?),
            "--strip" => options.strip = true,
            "--at" => options.at = Some(value(&mut index)?),
            "--start" => options.start = Some(value(&mut index)?),
            "--end" => options.end = Some(value(&mut index)?),
            "--rotate" => options.rotate = Some(value(&mut index)?.parse()?),
            "--title" => options.title = Some(value(&mut index)?),
            "--author" => options.author = Some(value(&mut index)?),
            "--watermark" => options.watermark = Some(value(&mut index)?),
            "--long" => options.long = true,
            "--sheet" => options.sheet = true,
            "--cover" => options.cover = true,
            "--gpu" => options.gpu = true,
            "--force" => options.force = true,
            "--merge" => options.merge = true,
            "--frames" => options.frames = true,
            other if other.starts_with('-') => {
                bail!("{} {other}", t("opcao desconhecida:", "unknown option:"))
            }
            other => inputs.push(PathBuf::from(other)),
        }
        index += 1;
    }

    if options.to.is_empty() {
        bail!("{}", t("falta --to <formato>", "missing --to <format>"));
    }
    if inputs.is_empty() {
        bail!("{}", t("nenhum arquivo informado", "no input files"));
    }
    Ok(Command::Convert { options, inputs })
}

fn parse_size(text: &str) -> Result<(u32, u32)> {
    let lower = text.to_lowercase();
    match lower.split_once('x') {
        Some((width, height)) => Ok((width.parse()?, height.parse()?)),
        None => Ok((lower.parse()?, 0)),
    }
}

pub fn help() -> String {
    format!(
        "SETTConverter {}\n\n{}\n\n{}\n\
         \x20        --at 00:01:30 --start 10 --end 60 --gpu --force\n\
         \x20        --width 1600 --dpi 300 --long --sheet\n\
         \x20        --rotate 90 --title TEXTO --author TEXTO --watermark TEXTO\n\n\
         {}: SETTConverter menu install | uninstall\n\
         {}: SETTConverter\\Settings\\SETTConverter.toml ({})\n\
         {}: %LOCALAPPDATA%\\SETTConverter\\tools",
        env!("CARGO_PKG_VERSION"),
        formats_help(),
        t(
            "Opcoes: --out DIR --size 1920x1080 --quality 1-100 --strip",
            "Options: --out DIR --size 1920x1080 --quality 1-100 --strip"
        ),
        t("Menu do Explorer", "Explorer menu"),
        t("Config", "Config"),
        t(
            "qualidade, PDF, video, audio, pastas e ferramentas",
            "quality, PDF, video, audio, folders and tools"
        ),
        t("FFmpeg e 7-Zip", "FFmpeg and 7-Zip"),
    )
}

pub fn formats_help() -> String {
    use formats::Family::*;
    let mut text = String::from(t("alvos por familia:", "targets by family:"));
    for family in [
        Image, Vector, Pdf, Video, Audio, Subtitle, Document, Font, Archive,
    ] {
        let targets: Vec<&str> = formats::menu_targets(family)
            .iter()
            .map(|(_, _, target)| *target)
            .collect();
        text.push_str(&format!("\n  {family:?}: {}", targets.join(" ")));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn convert_reads_flags_and_inputs() {
        let Command::Convert { options, inputs } = parse(&args(&[
            "convert", "--to", "JPG", "--size", "64x32", "--force", "a.png", "b.png",
        ]))
        .unwrap() else {
            panic!("expected convert");
        };
        assert_eq!(options.to, "jpg");
        assert_eq!(options.size, Some((64, 32)));
        assert!(options.force);
        assert_eq!(inputs.len(), 2);
    }

    #[test]
    fn extract_accepts_out() {
        let Command::Extract { options, inputs } =
            parse(&args(&["extract", "--out", "dest", "a.zip"])).unwrap()
        else {
            panic!("expected extract");
        };
        assert_eq!(options.to, "extract");
        assert_eq!(options.out.as_deref(), Some(std::path::Path::new("dest")));
        assert_eq!(inputs.len(), 1);
    }

    #[test]
    fn missing_or_unknown_arguments_are_errors() {
        assert!(parse(&args(&["convert", "a.png"])).is_err());
        assert!(parse(&args(&["convert", "--to", "jpg"])).is_err());
        assert!(parse(&args(&["convert", "--to", "jpg", "--nope", "a.png"])).is_err());
        assert!(parse(&args(&["bogus"])).is_err());
    }

    #[test]
    fn menu_and_help_commands() {
        assert!(matches!(
            parse(&args(&["menu", "install"])).unwrap(),
            Command::MenuInstall
        ));
        assert!(matches!(parse(&args(&[])).unwrap(), Command::Help));
        assert!(matches!(
            parse(&args(&["version"])).unwrap(),
            Command::Version
        ));
    }
}
