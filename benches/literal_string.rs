use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use sayit::utils::LiteralString;
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

// this is 100 times slower than _fast test
fn literal_string_slow(c: &mut Criterion) {
    let mut g = c.benchmark_group("literal_string");
    g.sample_size(500);

    g.bench_function("creation", |b| {
        let words = read_sample_words();

        b.iter(|| {
            for word in &words {
                let _ = LiteralString::from(word.as_str());
            }
        })
    });
    g.finish();
}

fn literal_string_fast(c: &mut Criterion) {
    let mut g = c.benchmark_group("literal_string");
    g.sample_size(300);

    g.bench_function("mimic_case", |b| {
        let words = read_sample_words();
        let strings: Vec<LiteralString> = words
            .iter()
            .map(|w| LiteralString::from(w.as_str()))
            .collect();
        let reversed_words: Vec<String> = words.into_iter().rev().collect();

        b.iter(|| {
            for (string, word) in strings.iter().zip(&reversed_words) {
                let _ = string.mimic_ascii_case(word);
            }
        })
    });
    g.finish();
}

criterion_group!(benches, literal_string_slow, literal_string_fast);
criterion_main!(benches);
