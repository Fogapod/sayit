mod utils;

use sayit::Accent;
use std::{fs, path::PathBuf};
use utils::read_sample_file_lines;

pub fn read_accent(filename: PathBuf) -> Accent {
    let content = fs::read_to_string(&filename).unwrap();
    serde_json::from_str::<Accent>(&content)
        .expect(&format!("parsing accent {}", filename.display()))
}

#[test]
fn json_examples_work() {
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
                accent.say_it(&line, intensity);
            }
        }
        tested_at_least_one = true;
    }

    assert!(tested_at_least_one);
}
