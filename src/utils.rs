use regex::Regex;

/// Wrapper around string, precomputing some metadata to speed up operations
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct SimpleString {
    pub(crate) body: String,
    pub(crate) has_template: bool,
    char_count: usize,
    is_ascii_only: bool,
    is_ascii_lowercase: bool,
    is_ascii_uppercase: bool,
}

impl SimpleString {
    pub(crate) fn new(body: &str) -> Self {
        // https://docs.rs/regex/latest/regex/struct.Captures.html#method.expand
        let template_regex = Regex::new(r"\$[0-9A-Za-z_]").unwrap();

        Self {
            body: body.to_owned(),
            char_count: body.chars().count(),
            is_ascii_only: body.is_ascii(),
            is_ascii_lowercase: body.chars().all(|c| c.is_ascii_lowercase()),
            is_ascii_uppercase: body.chars().all(|c| c.is_ascii_uppercase()),
            has_template: template_regex.is_match(body),
        }
    }
}

impl SimpleString {
    /// Try to learn something about strings and adjust case accordingly. all logic is currently
    /// ascii only
    pub(crate) fn mimic_ascii_case(&self, original: &str) -> String {
        let mut body = self.body.clone();

        // assume lowercase ascii is "weakest" form. anything else returns as is
        if !self.is_ascii_lowercase {
            return body;
        }

        // if original was all uppercase we force all uppercase for replacement. this is likely to
        // give false positives on short inputs like "I" or abbreviations
        if original.chars().all(|c| c.is_ascii_uppercase()) {
            return body.to_ascii_uppercase();
        }

        // no constraints if original was all lowercase
        if original
            .chars()
            .all(|c| !c.is_ascii() || c.is_ascii_lowercase())
        {
            return body;
        }

        // TODO: SIMD this
        if original.chars().count() == self.char_count {
            for (i, c_old) in original.chars().enumerate() {
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
        }

        body
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_detects_template() {
        assert!(!SimpleString::new("hello").has_template);
        assert!(SimpleString::new("$hello").has_template);
        assert!(SimpleString::new("hello $1 world").has_template);
    }

    #[test]
    fn string_counts_chars() {
        assert_eq!(SimpleString::new("hello").char_count, 5);
        assert_eq!(SimpleString::new("привет").char_count, 6);
    }

    #[test]
    fn string_detects_ascii_only() {
        assert_eq!(SimpleString::new("Hello").is_ascii_only, true);
        assert_eq!(SimpleString::new("1!@$#$").is_ascii_only, true);
        assert_eq!(SimpleString::new("Привет").is_ascii_only, false);
    }

    #[test]
    fn string_detects_ascii_lowercase() {
        assert_eq!(SimpleString::new("hello").is_ascii_lowercase, true);
        assert_eq!(SimpleString::new("Hello").is_ascii_lowercase, false);
        assert_eq!(SimpleString::new("1!@$#$").is_ascii_lowercase, false);
        assert_eq!(SimpleString::new("привет").is_ascii_lowercase, false);
    }

    #[test]
    fn string_detects_ascii_uppercase() {
        assert_eq!(SimpleString::new("HELLO").is_ascii_uppercase, true);
        assert_eq!(SimpleString::new("Hello").is_ascii_uppercase, false);
        assert_eq!(SimpleString::new("1!@$#$").is_ascii_uppercase, false);
        assert_eq!(SimpleString::new("ПРИВЕТ").is_ascii_uppercase, false);
    }

    #[test]
    fn mimic_case_input_lowercase() {
        assert_eq!(SimpleString::new("bye").mimic_ascii_case("hello"), "bye");
        assert_eq!(SimpleString::new("Bye").mimic_ascii_case("hello"), "Bye");
        assert_eq!(SimpleString::new("bYE").mimic_ascii_case("hello"), "bYE");
    }

    // questionable rule, becomes overcomplicated
    // #[test]
    // fn mimic_case_input_titled() {
    //     assert_eq!(
    //         SimpleString::new("bye").steal_ascii_case("Hello"),
    //         "Bye"
    //     );
    //     // has case variation -- do not touch it
    //     assert_eq!(
    //         SimpleString::new("bYe").steal_ascii_case("Hello"),
    //         "bYe"
    //     );
    //     // not ascii uppercase
    //     assert_eq!(
    //         SimpleString::new("bye").steal_ascii_case("Привет"),
    //         "bye"
    //     );
    // }

    #[test]
    fn mimic_case_input_uppercase() {
        assert_eq!(SimpleString::new("bye").mimic_ascii_case("HELLO"), "BYE");
        // has case variation -- do not touch it
        assert_eq!(SimpleString::new("bYE").mimic_ascii_case("HELLO"), "bYE");
        // not ascii uppercase
        assert_eq!(SimpleString::new("bye").mimic_ascii_case("ПРИВЕТ"), "bye");
        assert_eq!(SimpleString::new("пока").mimic_ascii_case("HELLO"), "пока");
    }

    #[test]
    fn mimic_case_input_different_case() {
        assert_eq!(SimpleString::new("bye").mimic_ascii_case("hELLO"), "bye");
    }

    #[test]
    fn mimic_case_input_different_case_same_len() {
        assert_eq!(
            SimpleString::new("byeee").mimic_ascii_case("hELLO"),
            "bYEEE"
        );
        assert_eq!(SimpleString::new("bye").mimic_ascii_case("hI!"), "bYe");
        assert_eq!(SimpleString::new("Bye").mimic_ascii_case("hI!"), "Bye");
    }
}
