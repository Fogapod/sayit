use std::sync::OnceLock;

use regex_automata::meta::Regex;

static TEMPLATE_REGEX: OnceLock<Regex> = OnceLock::new();

// https://stackoverflow.com/a/38406885
fn title(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn count_cases(string: &str) -> (usize, usize) {
    string.chars().fold((0, 0), |(lower, upper), c| {
        let is_lower = c.is_lowercase();
        let is_upper = c.is_uppercase();

        (lower + usize::from(is_lower), upper + usize::from(is_upper))
    })
}

fn count_chars_and_cases(string: &str) -> (usize, usize, usize) {
    string.chars().fold((0, 0, 0), |(total, lower, upper), c| {
        let is_lower = c.is_lowercase();
        let is_upper = c.is_uppercase();

        (
            total + 1,
            lower + usize::from(is_lower),
            upper + usize::from(is_upper),
        )
    })
}

#[doc(hidden)] // pub for bench
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MimicAction {
    Title,
    Uppercase,
    Nothing,
}

/// Allows examining string case when provided with info about characters
#[doc(hidden)] // pub for bench
pub trait LiteralString {
    fn chars(&self) -> (usize, bool, bool);

    /// Examine given string and tell which action to take to match it's case
    #[must_use]
    fn mimic_case_action(&self, from: &str) -> MimicAction {
        let (self_char_count, self_has_lowercase, self_has_uppercase) = self.chars();

        // do nothing if current string is:
        // - has at least one uppercase letter
        // - has no letters
        if self_has_uppercase || !self_has_lowercase {
            return MimicAction::Nothing;
        }

        let (char_count, lowercase, uppercase) = count_chars_and_cases(from);

        // uppercase: has no lowercase letters and at least one uppercase letter
        if (lowercase == 0 && uppercase != 0)
            // either current string is 1 letter or string is upper and is long
            && (self_char_count == 1 || char_count > 1)
        {
            return MimicAction::Uppercase;
        }

        // there is exactly one uppercase letter
        if uppercase == 1
            // either one letter long or first letter is upper
            && (char_count == 1 || from.chars().next().is_some_and(char::is_uppercase))
        {
            return MimicAction::Title;
        }

        MimicAction::Nothing
    }
}

/// Wrapper around string. Performs single case mimicking, does not precompute anything
pub(crate) struct LazyLiteral {
    body: String,
    length_hint: usize,
}

impl LazyLiteral {
    pub(crate) fn new(body: String, length_hint: usize) -> Self {
        Self { body, length_hint }
    }

    pub(crate) fn handle_mimic_action(self, action: MimicAction) -> String {
        match action {
            MimicAction::Title => title(&self.body),
            MimicAction::Uppercase => self.body.to_uppercase(),
            MimicAction::Nothing => self.body,
        }
    }
}

impl LiteralString for LazyLiteral {
    fn chars(&self) -> (usize, bool, bool) {
        let (lowercase, uppercase) = count_cases(&self.body);

        (self.length_hint, lowercase != 0, uppercase != 0)
    }
}

/// Wrapper around string. Optionally precomputes information for fast case mimicking
#[doc(hidden)] // pub for bench
#[derive(Debug, Clone)]
pub struct PrecomputedLiteral {
    pub(crate) body: String,
    body_upper: String,
    body_title: String,
    pub(crate) has_template: bool,
    char_count: usize,
    has_lowercase: bool,
    has_uppercase: bool,
}

impl PrecomputedLiteral {
    #[doc(hidden)] // pub for bench
    pub fn new(body: String) -> Self {
        let (char_count, lowercase, uppercase) = count_chars_and_cases(&body);

        Self {
            char_count,
            has_lowercase: lowercase != 0,
            has_uppercase: uppercase != 0,
            body_upper: body.to_uppercase(),
            body_title: title(&body),
            // https://docs.rs/regex-automata/latest/regex_automata/util/interpolate/index.html
            // this is not 100% accurate but should never result in false negatives
            has_template: TEMPLATE_REGEX
                .get_or_init(|| Regex::new(r"(:?^|[^$])\$(:?[0-9A-Za-z_]|\{.+?\})").unwrap())
                .is_match(&body),
            body,
        }
    }

    pub(crate) fn handle_mimic_action(&self, action: MimicAction) -> String {
        match action {
            MimicAction::Title => self.body_title.clone(),
            MimicAction::Uppercase => self.body_upper.clone(),
            MimicAction::Nothing => self.body.clone(),
        }
    }
}

impl LiteralString for PrecomputedLiteral {
    fn chars(&self) -> (usize, bool, bool) {
        (self.char_count, self.has_lowercase, self.has_uppercase)
    }
}

impl PartialEq for PrecomputedLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.body == other.body
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

    impl From<&str> for PrecomputedLiteral {
        fn from(body: &str) -> Self {
            Self::new(body.to_string())
        }
    }

    #[test]
    fn string_detects_template() {
        assert!(!PrecomputedLiteral::from("hello").has_template);
        assert!(PrecomputedLiteral::from("$hello").has_template);
        assert!(PrecomputedLiteral::from("hello $1 world").has_template);
        assert!(!PrecomputedLiteral::from("hello $$1 world").has_template);
        assert!(!PrecomputedLiteral::from("hello $$$1 world").has_template);
        assert!(PrecomputedLiteral::from("hello ${foo[bar].baz} world").has_template);
        assert!(!PrecomputedLiteral::from("hello $${foo[bar].baz} world").has_template);
    }

    #[test]
    fn string_counts_chars() {
        assert_eq!(PrecomputedLiteral::from("hello").chars().0, 5);
        assert_eq!(PrecomputedLiteral::from("привет").chars().0, 6);
    }

    #[test]
    fn string_detects_lowercase() {
        assert_eq!(PrecomputedLiteral::from("hello").chars().1, true);
        assert_eq!(PrecomputedLiteral::from("Hello").chars().1, true);
        assert_eq!(PrecomputedLiteral::from("1!@$#$").chars().1, false);
        assert_eq!(PrecomputedLiteral::from("1!@$H#$").chars().1, false);
        assert_eq!(PrecomputedLiteral::from("1!@$Hh#$").chars().1, true);
        assert_eq!(PrecomputedLiteral::from("привет").chars().1, true);
        assert_eq!(PrecomputedLiteral::from("ПРИВЕТ").chars().1, false);
    }
    #[test]
    fn string_detects_uppercase() {
        assert_eq!(PrecomputedLiteral::from("hello").chars().2, false);
        assert_eq!(PrecomputedLiteral::from("Hello").chars().2, true);
        assert_eq!(PrecomputedLiteral::from("1!@$#$").chars().2, false);
        assert_eq!(PrecomputedLiteral::from("1!@$H#$").chars().2, true);
        assert_eq!(PrecomputedLiteral::from("1!@$Hh#$").chars().2, true);
        assert_eq!(PrecomputedLiteral::from("привет").chars().2, false);
        assert_eq!(PrecomputedLiteral::from("ПРИВЕТ").chars().2, true);
    }

    #[test]
    fn mimic_case_input_lowercase() {
        assert_eq!(
            PrecomputedLiteral::from("bye").mimic_case_action("hello"),
            MimicAction::Nothing
        );
        assert_eq!(
            PrecomputedLiteral::from("Bye").mimic_case_action("hello"),
            MimicAction::Nothing
        );
        assert_eq!(
            PrecomputedLiteral::from("bYE").mimic_case_action("hello"),
            MimicAction::Nothing
        );
    }

    #[test]
    fn mimic_case_input_titled() {
        assert_eq!(
            PrecomputedLiteral::from("bye").mimic_case_action("Hello"),
            MimicAction::Title
        );
        // has case variation -- do not touch it
        assert_eq!(
            PrecomputedLiteral::from("bYe").mimic_case_action("Hello"),
            MimicAction::Nothing
        );
        // non ascii title
        assert_eq!(
            PrecomputedLiteral::from("bye").mimic_case_action("Привет"),
            MimicAction::Title
        );
    }
    #[test]
    fn mimic_case_input_titled_single_letter() {
        assert_eq!(
            PrecomputedLiteral::from("je").mimic_case_action("I"),
            MimicAction::Title
        );
    }

    #[test]
    fn mimic_case_input_uppercase() {
        assert_eq!(
            PrecomputedLiteral::from("bye").mimic_case_action("HELLO"),
            MimicAction::Uppercase
        );
        // has case variation -- do not touch it
        assert_eq!(
            PrecomputedLiteral::from("bbbbYE").mimic_case_action("HELLO"),
            MimicAction::Nothing
        );
        // non ascii uppercase
        assert_eq!(
            PrecomputedLiteral::from("bye").mimic_case_action("ПРИВЕТ"),
            MimicAction::Uppercase
        );
        assert_eq!(
            PrecomputedLiteral::from("пока").mimic_case_action("HELLO"),
            MimicAction::Uppercase
        );
    }

    #[test]
    fn mimic_case_input_mixed_case() {
        assert_eq!(
            PrecomputedLiteral::from("bye").mimic_case_action("hELLO"),
            MimicAction::Nothing
        );
        assert_eq!(
            PrecomputedLiteral::from("пока").mimic_case_action("HEllo"),
            MimicAction::Nothing
        );
        assert_eq!(
            PrecomputedLiteral::from("пока").mimic_case_action("HELlo"),
            MimicAction::Nothing
        );
        assert_eq!(
            PrecomputedLiteral::from("bye").mimic_case_action("heLlo"),
            MimicAction::Nothing
        );
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
