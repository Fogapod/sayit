/// Wrapper around string, precomputing some metadata to speed up operations
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct SimpleString {
    pub(crate) body: String,
    char_count: usize,
    is_ascii_only: bool,
    is_ascii_lowercase: bool,
    is_ascii_uppercase: bool,
}

impl SimpleString {
    pub(crate) fn new(body: &str) -> Self {
        Self {
            body: body.to_owned(),
            char_count: body.chars().count(),
            is_ascii_only: body.is_ascii(),
            is_ascii_lowercase: body.chars().all(|c| c.is_ascii_lowercase()),
            is_ascii_uppercase: body.chars().all(|c| c.is_ascii_uppercase()),
        }
    }
}

/// try to learn something about strings and adjust case accordingly. all logic is currently
/// ascii only
/// tried using Cows but my computer exploded. TODO: try that again
pub(crate) fn normalize_case(old: &str, new: &SimpleString) -> String {
    let mut body = new.body.clone();

    // assume lowercase ascii is "weakest" form. anything else returns as is
    if !new.is_ascii_lowercase {
        return body;
    }

    // if original was all uppercase we force all uppercase for replacement. this is likely to
    // give false positives on short inputs like "I" or abbreviations
    if old.chars().all(|c| c.is_ascii_uppercase()) {
        return body.to_ascii_uppercase();
    }

    // no constraints if original was all lowercase
    if old.chars().all(|c| !c.is_ascii() || c.is_ascii_lowercase()) {
        return body;
    }

    if old.chars().count() == new.char_count {
        for (i, c_old) in old.chars().enumerate() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_case_input_lowercase() {
        assert_eq!(normalize_case("hello", &SimpleString::new("bye")), "bye");
        assert_eq!(normalize_case("hello", &SimpleString::new("Bye")), "Bye");
        assert_eq!(normalize_case("hello", &SimpleString::new("bYE")), "bYE");
    }

    // questionable rule, becomes overcomplicated
    // #[test]
    // fn normalize_case_input_titled() {
    //     assert_eq!(
    //         normalize_case("Hello", &SimpleString::new("bye")),
    //         "Bye"
    //     );
    //     // has case variation -- do not touch it
    //     assert_eq!(
    //         normalize_case("Hello", &SimpleString::new("bYe")),
    //         "bYe"
    //     );
    //     // not ascii uppercase
    //     assert_eq!(
    //         normalize_case("Привет", &SimpleString::new("bye")),
    //         "bye"
    //     );
    // }

    #[test]
    fn normalize_case_input_uppercase() {
        assert_eq!(normalize_case("HELLO", &SimpleString::new("bye")), "BYE");
        // has case variation -- do not touch it
        assert_eq!(normalize_case("HELLO", &SimpleString::new("bYE")), "bYE");
        // not ascii uppercase
        assert_eq!(normalize_case("ПРИВЕТ", &SimpleString::new("bye")), "bye");
        assert_eq!(normalize_case("HELLO", &SimpleString::new("пока")), "пока");
    }

    #[test]
    fn normalize_case_input_different_case() {
        assert_eq!(normalize_case("hELLO", &SimpleString::new("bye")), "bye");
    }

    #[test]
    fn normalize_case_input_different_case_same_len() {
        assert_eq!(
            normalize_case("hELLO", &SimpleString::new("byeee")),
            "bYEEE"
        );
        assert_eq!(normalize_case("hI!", &SimpleString::new("bye")), "bYe");
        assert_eq!(normalize_case("hI!", &SimpleString::new("Bye")), "Bye");
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
}
