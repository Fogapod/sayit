use sayit::Accent;

use std::{fs, io, path::PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Read accent from file (currently only ron supported)
    #[arg(short, long, group = "accent_def")]
    accent: Option<PathBuf>,

    /// Read accent from stdin(currently only ron supported)
    #[arg(long, group = "accent_def")]
    accent_string: Option<String>,

    /// Set intensity
    #[arg(short, long, default_value_t = 0)]
    intensity: u64,
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

    for line in io::stdin().lines() {
        let line = line.map_err(|err| format!("reading line: {err}"))?;
        let applied = accent.say_it(&line, args.intensity);

        println!("{applied}");
    }

    Ok(())
}
