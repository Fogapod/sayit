mod utils;

use sayit::Accent;
use serde::Deserialize;
use std::{fs, path::PathBuf};
use utils::read_sample_file_lines;

pub fn read_accent(filename: PathBuf) -> Accent {
    let content = fs::read_to_string(&filename).unwrap();
    ron::from_str::<Accent>(&content)
        .unwrap_or_else(|err| panic!("parsing accent {}: {err}", filename.display()))
}

// flatten breaks for unknown reason, possibly related to json failure, see json.rs.
// it sees sequence somewhere around Extend
// https://github.com/serde-rs/serde/issues/1183
#[test]
#[should_panic(expected = r#"expected: "string or map", found: "a sequence"#)]
fn flatten_broken() {
    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(flatten)]
        accent: Accent,
    }

    let accent = ron::from_str::<Wrapper>(
        r#"
{
    accent: {},
    intensities: {
        1: Extend({}),
    },
}
"#,
    )
    .unwrap();

    println!("{}", accent.accent.say_it("hello world", 0));
}

#[test]
fn ron_accents_work() {
    let lines = read_sample_file_lines();

    let mut tested_at_least_one = false;

    for directory in ["examples", "benches"] {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();

            if !path.is_file() {
                continue;
            }

            if !path.extension().is_some_and(|ext| ext == "ron") {
                continue;
            }

            println!("running {}", path.display());

            let accent = read_accent(path);
            for line in &lines {
                for intensity in accent.intensities() {
                    let _ = accent.say_it(line, intensity);
                }
            }
            tested_at_least_one = true;
        }
    }

    assert!(tested_at_least_one);
}
