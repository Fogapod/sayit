mod utils;

use sayit::Accent;
use serde::Deserialize;
use std::{fs, path::PathBuf};
use utils::read_sample_file_lines;

pub fn read_accent(filename: PathBuf) -> Accent {
    let content = fs::read_to_string(&filename).unwrap();
    serde_json::from_str::<Accent>(&content)
        .unwrap_or_else(|_| panic!("parsing accent {}", filename.display()))
}

// flatten breaks treating string map keys as u64 (ints are not allowed as json map keys)
// https://github.com/serde-rs/serde/issues/1183
#[test]
#[should_panic(expected = "expected u64")]
fn flatten_broken() {
    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(flatten)]
        accent: Accent,
    }

    let accent = serde_json::from_str::<Wrapper>(
        r#"
{
    "accent": {},
    "intensities": {
        "1": {"Extend": {
            "main": {
                "rules": {}
            }
        }}
    }
}
"#,
    )
    .unwrap();

    println!("{}", accent.accent.say_it("hello world", 0));
}

#[test]
fn json_accents_work() {
    let lines = read_sample_file_lines();

    let mut tested_at_least_one = false;

    for entry in fs::read_dir("examples").unwrap() {
        let path = entry.unwrap().path();

        if !path.is_file() {
            continue;
        }

        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }

        println!("running {}", path.display());
        let accent = read_accent(path);
        for line in &lines {
            for intensity in accent.intensities() {
                accent.say_it(line, intensity);
            }
        }
        tested_at_least_one = true;
    }

    assert!(tested_at_least_one);
}
