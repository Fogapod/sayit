use criterion::{criterion_group, criterion_main, Criterion};
use sayit::Accent;
use std::{fs, path::PathBuf};

pub fn read_accent(filename: &PathBuf) -> Accent {
    let content = fs::read_to_string(filename).expect("reading accent definition");
    ron::from_str::<Accent>(&content)
        .unwrap_or_else(|_| panic!("parsing accent {}", filename.display()))
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

fn examples(c: &mut Criterion) {
    let lines = read_sample_file_lines();

    let mut g = c.benchmark_group("examples");
    g.sampling_mode(criterion::SamplingMode::Linear);

    for entry in fs::read_dir("examples").unwrap() {
        let path = entry.unwrap().path();

        if !path.is_file() {
            continue;
        }

        if !path.extension().is_some_and(|ext| ext == "ron") {
            continue;
        }

        let accent = read_accent(&path);
        let accent_name = path.file_stem().unwrap().to_string_lossy();

        g.bench_function(accent_name, |b| {
            fastrand::seed(0);

            b.iter(|| {
                for line in &lines {
                    let _ = accent.say_it(line, 0);
                }
            })
        });
    }
    g.finish();
}

criterion_group!(benches, examples);
criterion_main!(benches);
