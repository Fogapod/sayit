use criterion::criterion_main;
use criterion::{criterion_group, Criterion};
use sayit::Accent;
use std::fs;

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

    let mut g = c.benchmark_group("accents");
    g.sample_size(2000);

    for name in [
        "original", "literal", "any", "weights", "upper", "lower", "concat",
    ] {
        let accent = read_accent(&format!("benches/{name}.ron"));

        g.bench_function(name, |b| {
            b.iter(|| {
                for line in &lines {
                    accent.apply(&line, 0);
                }
            })
        });
    }
    g.finish();
}

criterion_group!(benches, accents);
criterion_main!(benches);
