use std::fs;

/// Read sample text into string
pub fn read_sample_file() -> String {
    fs::read_to_string("tests/sample_text.txt").unwrap()
}

/// Read sample text lines and filter junk
pub fn read_sample_file_lines() -> Vec<String> {
    read_sample_file()
        .lines()
        .filter(|&l| !(l.is_empty() || l.eq(" :")))
        .map(|s| s.to_owned())
        .collect()
}
