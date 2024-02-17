use std::sync::OnceLock;

use regex_automata::meta::Regex;

/// Wrapper around string, precomputing some metadata to speed up operations
///
/// NOTE: this is very expensive to initialyze
#[doc(hidden)] // pub for bench
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(from = "&str")
)]
pub struct LiteralString {
    pub(crate) body: String,
    // templating is expensive, it is important to skip it if possible
    pub(crate) has_template: bool,
    // saves time in mimic_case
    char_count: usize,
    is_ascii_lowercase: bool,
}

impl PartialEq for LiteralString {
    fn eq(&self, other: &Self) -> bool {
        self.body == other.body
    }
}

fn case(char_count: usize, string: &str) -> (bool, bool, bool) {
    let (lower, upper) = string.chars().fold((0, 0), |(lower, upper), c| {
        (
            lower + usize::from(c.is_ascii_lowercase()),
            upper + usize::from(c.is_ascii_uppercase()),
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

        let (is_ascii_lowercase, _is_ascii_uppercase, _is_ascii_mixed_case) =
            case(char_count, body);

        Self {
            body: body.to_owned(),
            char_count,
            is_ascii_lowercase,
            has_template: TEMPLATE_REGEX
                // https://docs.rs/regex-automata/latest/regex_automata/util/interpolate/index.html
                // this is not 100% accurate but should never result in false negatives
                .get_or_init(|| Regex::new(r"(:?^|[^$])\$(:?[0-9A-Za-z_]|\{.+?\})").unwrap())
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
        if source.chars().all(|c| c.is_ascii_lowercase()) || !source.is_ascii() {
            return self.body.clone();
        }

        // TODO: SIMD this
        if source.chars().count() == self.char_count {
            let mut body = self.body.clone();

            for (i, c_old) in source.chars().enumerate() {
                if c_old.is_ascii_lowercase() {
                    body.get_mut(i..=i)
                        .expect("strings have same len")
                        .make_ascii_lowercase();
                } else if c_old.is_ascii_uppercase() {
                    body.get_mut(i..=i)
                        .expect("strings have same len")
                        .make_ascii_uppercase();
                }
            }

            return body;
        }

        self.body.clone()
    }
}

// replaces a SINGLE REQUIRED "{}" template in string. braces can be escaped by doubling "{{" "}}"
pub(crate) fn runtime_format_single_value(template: &str, value: &str) -> Result<String, String> {
    let mut result = String::new();

    let mut formatted = false;
    let mut previous = None;

    for (i, c) in template.chars().enumerate() {
        match c {
            '{' => {
                if let Some('{') = previous {
                    result.push('{');
                    previous = None;
                } else {
                    previous = Some('{');
                }
            }
            '}' => match (previous, formatted) {
                (Some('{'), true) => return Err(format!("unmatched '{{' at position {i}")),
                (Some('{'), false) => {
                    result.push_str(value);
                    formatted = true;
                    previous = None;
                }
                (Some('}'), _) => {
                    result.push('}');
                    previous = None;
                }
                (None, _) => previous = Some('}'),
                (Some(_), _) => unreachable!(),
            },
            _ => {
                if let Some(previous) = previous {
                    return Err(format!("unmatched '{previous}' at position {i}"));
                }

                result.push(c);
            }
        }
    }

    if !formatted {
        return Err("string did not contain {} template".to_owned());
    }

    Ok(result)
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
    fn string_detects_ascii_lowercase() {
        assert_eq!(LiteralString::from("hello").is_ascii_lowercase, true);
        assert_eq!(LiteralString::from("Hello").is_ascii_lowercase, false);
        assert_eq!(LiteralString::from("1!@$#$").is_ascii_lowercase, false);
        assert_eq!(LiteralString::from("привет").is_ascii_lowercase, false);
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

    #[test]
    fn runtime_format_formats() {
        assert_eq!(runtime_format_single_value("{}", "1").unwrap(), "1");
        assert_eq!(runtime_format_single_value(" {}", "2").unwrap(), " 2");
        assert_eq!(runtime_format_single_value("{} ", "3").unwrap(), "3 ");
    }

    #[test]
    fn runtime_format_escapes() {
        assert_eq!(
            runtime_format_single_value("}} {{{}}}", "1").unwrap(),
            "} {1}"
        );
    }

    #[test]
    fn runtime_format_requires_replacement() {
        assert!(runtime_format_single_value("hello {{", "world").is_err());
    }

    #[test]
    fn runtime_format_one_replacement() {
        assert!(runtime_format_single_value("hello {} {}", "world").is_err());
    }

    #[test]
    fn runtime_format_unmatched() {
        assert!(runtime_format_single_value("}0", "world").is_err());
        assert!(runtime_format_single_value("{0", "world").is_err());
    }
}
