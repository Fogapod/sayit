use criterion::criterion_main;
use criterion::{criterion_group, Criterion};
use pink_accents::Accent;
use std::fs;
use std::time::Duration;

pub fn read_accent(filename: &str) -> Accent {
    let content = fs::read_to_string(filename).expect("reading accent definition");
    ron::from_str::<Accent>(&content).expect("parsing accent")
}

pub fn read_sample_file() -> String {
    fs::read_to_string("tests/sample_text.txt").expect("reading sample text")
}

pub fn read_sample_file_lines() -> Vec<String> {
    read_sample_file()
        .lines()
        .filter(|&l| !(l.is_empty() || l.eq(" :")))
        .map(|s| s.to_owned())
        .collect()
}

fn accents(c: &mut Criterion) {
    let lines = read_sample_file_lines();

    for replacement in [
        "original",
        "simple",
        "any",
        "weights",
        "uppercase",
        "lowercase",
    ] {
        let accent = read_accent(&format!("benches/{replacement}.ron"));

        c.bench_function(&format!("accents::{replacement}"), |b| {
            b.iter(|| {
                for line in &lines {
                    accent.apply(&line, 0);
                }
            })
        });
    }
}

criterion_group!(
    name=benches;
    config=Criterion::default().measurement_time(Duration::from_secs(10));
    targets=accents
);

criterion_main!(benches);
