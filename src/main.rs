use sayit::Accent;

use std::{borrow::Cow, fs, io, path::PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Read accent from file (currently only ron supported)
    #[arg(short, long, group = "accent_def")]
    accent: Vec<PathBuf>,

    /// Read accent from stdin(currently only ron supported)
    #[arg(long, group = "accent_def")]
    accent_string: Vec<String>,

    /// Set intensity
    #[arg(short, long)]
    intensity: Vec<u64>,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    let accent_strings = if !args.accent.is_empty() {
        let mut strs = Vec::with_capacity(args.accent.len());
        for accent_path in args.accent {
            strs.push(fs::read_to_string(&accent_path).map_err(|err| {
                format!("reading accent {} file: {}", accent_path.display(), err)
            })?);
        }
        strs
    } else {
        // TODO: figure out how to make clap group do the check
        if args.accent_string.is_empty() {
            return Err("expected either --accent or --accent-string".to_owned());
        }

        args.accent_string
    };

    let intensities = if args.intensity.is_empty() {
        vec![0; accent_strings.len()]
    } else {
        args.intensity
    };

    if accent_strings.len() != intensities.len() {
        return Err("different number of accents and intensities provided".to_owned());
    }

    let mut accents = Vec::with_capacity(accent_strings.len());
    for accent_str in accent_strings {
        accents.push(
            ron::from_str::<Accent>(&accent_str).map_err(|err| format!("parsing accent: {err}"))?,
        );
    }

    for line in io::stdin().lines() {
        let line = &line.map_err(|err| format!("reading line: {err}"))?;

        let applied = accents
            .iter()
            .zip(intensities.iter())
            .fold(Cow::Borrowed(line), |acc, (accent, &level)| {
                Cow::Owned(accent.say_it(&acc, level).into_owned())
            });

        println!("{applied}");
    }

    Ok(())
}
