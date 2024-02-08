use std::sync::OnceLock;

use regex::Regex;

/// Wrapper around string, precomputing some metadata to speed up operations
///
/// NOTE: this is very expensive to initialyze
#[doc(hidden)] // pub for bench
#[derive(Debug, PartialEq, Clone)]
pub struct LiteralString {
    pub(crate) body: String,
    // templating is expensive, it is important to skip it if possible
    pub(crate) has_template: bool,
    // saves time in mimic_case
    char_count: usize,
    is_ascii_only: bool,
    is_ascii_lowercase: bool,
    is_ascii_uppercase: bool,
    is_ascii_mixed_case: bool,
}

fn case(char_count: usize, string: &str) -> (bool, bool, bool) {
    let (lower, upper) = string.chars().fold((0, 0), |(lower, upper), c| {
        (
            lower + c.is_ascii_lowercase() as usize,
            upper + c.is_ascii_uppercase() as usize,
        )
    });

    (
        lower == char_count,
        upper == char_count,
        lower > 0 && upper > 0,
    )
}

static TEMPLATE_REGEX: OnceLock<Regex> = OnceLock::new();

impl From<&str> for LiteralString {
    fn from(body: &str) -> Self {
        let char_count = body.chars().count();
        let is_ascii_only = body.is_ascii();

        let (is_ascii_lowercase, is_ascii_uppercase, is_ascii_mixed_case) = case(char_count, body);

        Self {
            body: body.to_owned(),
            char_count,
            is_ascii_only,
            is_ascii_lowercase,
            is_ascii_uppercase,
            is_ascii_mixed_case,
            has_template: TEMPLATE_REGEX
                // https://docs.rs/regex/latest/regex/struct.Captures.html#method.expand
                // this is not 100% accurate but should never result in false negatives
                .get_or_init(|| Regex::new(r"(^|[^$])\$([0-9A-Za-z_]|\{.+?\})").unwrap())
                .is_match(body),
        }
    }
}

impl LiteralString {
    /// Examine given string and try to adjust to it's case. ascii only
    #[doc(hidden)] // pub for bench
    pub fn mimic_ascii_case(&self, source: &str) -> String {
        // only entirely lowercased string is changed. assume case has meaning for everything else
        if !self.is_ascii_lowercase {
            return self.body.clone();
        }

        // if source was all uppercase we force all uppercase for replacement. this is likely to
        // give false positives on short inputs like "I" or abbreviations
        if source.chars().all(|c| c.is_ascii_uppercase()) {
            return self.body.to_ascii_uppercase();
        }

        // no constraints if source was all lowercase
        if source
            .chars()
            .all(|c| !c.is_ascii() || c.is_ascii_lowercase())
        {
            return self.body.clone();
        }

        // TODO: SIMD this
        if source.chars().count() == self.char_count {
            let mut body = self.body.clone();

            for (i, c_old) in source.chars().enumerate() {
                if c_old.is_ascii_lowercase() {
                    body.get_mut(i..i + 1)
                        .expect("strings have same len")
                        .make_ascii_lowercase()
                } else if c_old.is_ascii_uppercase() {
                    body.get_mut(i..i + 1)
                        .expect("strings have same len")
                        .make_ascii_uppercase()
                }
            }

            return body;
        }

        self.body.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_detects_template() {
        assert!(!LiteralString::from("hello").has_template);
        assert!(LiteralString::from("$hello").has_template);
        assert!(LiteralString::from("hello $1 world").has_template);
        assert!(!LiteralString::from("hello $$1 world").has_template);
        assert!(!LiteralString::from("hello $$$1 world").has_template);
        assert!(LiteralString::from("hello ${foo[bar].baz} world").has_template);
        assert!(!LiteralString::from("hello $${foo[bar].baz} world").has_template);
    }

    #[test]
    fn string_counts_chars() {
        assert_eq!(LiteralString::from("hello").char_count, 5);
        assert_eq!(LiteralString::from("привет").char_count, 6);
    }

    #[test]
    fn string_detects_ascii_only() {
        assert_eq!(LiteralString::from("Hello").is_ascii_only, true);
        assert_eq!(LiteralString::from("1!@$#$").is_ascii_only, true);
        assert_eq!(LiteralString::from("Привет").is_ascii_only, false);
    }

    #[test]
    fn string_detects_ascii_lowercase() {
        assert_eq!(LiteralString::from("hello").is_ascii_lowercase, true);
        assert_eq!(LiteralString::from("Hello").is_ascii_lowercase, false);
        assert_eq!(LiteralString::from("1!@$#$").is_ascii_lowercase, false);
        assert_eq!(LiteralString::from("привет").is_ascii_lowercase, false);
    }

    #[test]
    fn string_detects_ascii_uppercase() {
        assert_eq!(LiteralString::from("HELLO").is_ascii_uppercase, true);
        assert_eq!(LiteralString::from("Hello").is_ascii_uppercase, false);
        assert_eq!(LiteralString::from("1!@$#$").is_ascii_uppercase, false);
        assert_eq!(LiteralString::from("ПРИВЕТ").is_ascii_uppercase, false);
    }

    #[test]
    fn mimic_case_input_lowercase() {
        assert_eq!(LiteralString::from("bye").mimic_ascii_case("hello"), "bye");
        assert_eq!(LiteralString::from("Bye").mimic_ascii_case("hello"), "Bye");
        assert_eq!(LiteralString::from("bYE").mimic_ascii_case("hello"), "bYE");
    }

    // questionable rule, becomes overcomplicated
    // #[test]
    // fn mimic_case_input_titled() {
    //     assert_eq!(LiteralString::from("bye").mimic_ascii_case("Hello"), "Bye");
    //     // has case variation -- do not touch it
    //     assert_eq!(LiteralString::from("bYe").mimic_ascii_case("Hello"), "bYe");
    //     // not ascii uppercase
    //     assert_eq!(LiteralString::from("bye").mimic_ascii_case("Привет"), "bye");
    // }

    #[test]
    fn mimic_case_input_uppercase() {
        assert_eq!(LiteralString::from("bye").mimic_ascii_case("HELLO"), "BYE");
        // has case variation -- do not touch it
        assert_eq!(LiteralString::from("bYE").mimic_ascii_case("HELLO"), "bYE");
        // not ascii uppercase
        assert_eq!(LiteralString::from("bye").mimic_ascii_case("ПРИВЕТ"), "bye");
        assert_eq!(
            LiteralString::from("пока").mimic_ascii_case("HELLO"),
            "пока"
        );
    }

    #[test]
    fn mimic_case_input_different_case() {
        assert_eq!(LiteralString::from("bye").mimic_ascii_case("hELLO"), "bye");
    }

    #[test]
    fn mimic_case_input_different_case_same_len() {
        assert_eq!(
            LiteralString::from("byeee").mimic_ascii_case("hELLO"),
            "bYEEE"
        );
        assert_eq!(LiteralString::from("bye").mimic_ascii_case("hI!"), "bYe");
        assert_eq!(LiteralString::from("Bye").mimic_ascii_case("hI!"), "Bye");
    }
}
