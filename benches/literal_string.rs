use criterion::{criterion_group, criterion_main, Criterion, SamplingMode};
use sayit::utils::{LiteralString, PrecomputedLiteral};
use std::fs;

pub fn read_sample_file() -> String {
    fs::read_to_string("tests/sample_text.txt").expect("reading sample text")
}

fn read_sample_words() -> Vec<String> {
    read_sample_file()
        .split_whitespace()
        // there are a lot of " :" lines for some reason. delete them
        .filter(|&w| w != ":")
        // remove direct speech colon: "Adam:" -> "Adam"
        .map(|w| w.strip_suffix(':').unwrap_or(w))
        .map(|w| w.to_owned())
        .collect()
}

fn literal_string(c: &mut Criterion) {
    let mut g = c.benchmark_group("literal_string");
    g.sampling_mode(SamplingMode::Linear);

    g.bench_function("mimic_case", |b| {
        let words = read_sample_words();
        let strings: Vec<PrecomputedLiteral> = words
            .iter()
            .map(|w| PrecomputedLiteral::new(w.to_string()))
            .collect();
        let reversed_words: Vec<String> = words.into_iter().rev().collect();

        b.iter(|| {
            for (string, word) in strings.iter().zip(&reversed_words) {
                let _ = string.mimic_case_action(word);
            }
        })
    });
    g.finish();
}

criterion_group!(benches, literal_string);
criterion_main!(benches);
