use std::{
    fs::{self, File},
    io::{self, BufRead},
    path::PathBuf,
};

use clap::Parser;

use pink_accents::Accent;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Accent file path (currently only ron supported)
    #[arg(short, long, group = "accent_def")]
    accent: Option<PathBuf>,

    /// Directly provided accent (ron format)
    #[arg(long, group = "accent_def")]
    accent_string: Option<String>,

    /// Accent severity
    #[arg(short, long, default_value_t = 0)]
    severity: u64,

    /// File to apply accent to. Reads from stdin if unset
    #[arg(short, long)]
    file: Option<PathBuf>,
}

fn apply_accent(accent: &Accent, severity: u64, line: io::Result<String>) -> Result<(), String> {
    println!(
        "{}",
        accent.apply(
            &line.map_err(|err| format!("reading line: {err}"))?,
            severity
        )
    );

    Ok(())
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    let accent_string = if let Some(accent) = args.accent {
        fs::read_to_string(accent).map_err(|err| format!("reading accent file: {err}"))?
    } else {
        // TODO: figure out how to make clap group do the check
        args.accent_string
            .ok_or_else(|| "expected either --accent or --accent-string")?
    };

    let accent =
        ron::from_str::<Accent>(&accent_string).map_err(|err| format!("parsing accent: {err}"))?;

    if let Some(filename) = args.file {
        let file = File::open(filename).map_err(|err| format!("reading input file: {err}"))?;
        for line in io::BufReader::new(file).lines() {
            apply_accent(&accent, args.severity, line)?;
        }
    } else {
        for line in io::stdin().lines() {
            apply_accent(&accent, args.severity, line)?;
        }
    }

    Ok(())
}
