use crate::utils::{count_chars_and_cases, to_title_case, LiteralString, MimicAction};
use std::{borrow::Cow, sync::OnceLock};

use regex_automata::meta::Regex;

use crate::{Match, Tag};

static TEMPLATE_REGEX: OnceLock<Regex> = OnceLock::new();

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
            body_title: to_title_case(&body),
            // https://docs.rs/regex-automata/latest/regex_automata/util/interpolate/index.html
            // this is not 100% accurate but should never result in false negatives
            has_template: TEMPLATE_REGEX
                .get_or_init(|| Regex::new(r"(:?^|[^$])\$(:?[0-9A-Za-z_]|\{.+?\})").unwrap())
                .is_match(&body),
            body,
        }
    }

    #[inline]
    pub(crate) fn handle_mimic_action(&self, action: MimicAction) -> Cow<'_, str> {
        match action {
            MimicAction::Title => &self.body_title,
            MimicAction::Uppercase => &self.body_upper,
            MimicAction::Nothing => &self.body,
        }
        .into()
    }
}

impl LiteralString for PrecomputedLiteral {
    #[inline]
    fn chars(&self) -> (usize, bool, bool) {
        (self.char_count, self.has_lowercase, self.has_uppercase)
    }
}

/// Static string
///
/// Acts as regex template, syntax doc: <https://docs.rs/regex/latest/regex/struct.Regex.html#example-9>
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Literal(PrecomputedLiteral);

impl Literal {
    pub fn new(s: String) -> Self {
        Self(PrecomputedLiteral::new(s))
    }

    // reference to simplify tests
    pub fn new_boxed(s: &str) -> Box<Self> {
        Box::new(Self::new(s.to_string()))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Literal {
    fn generate<'tag, 'inp: 'tag>(&'tag self, m: &Match<'inp>) -> Cow<'tag, str> {
        if self.0.has_template {
            let interpolated = m.interpolate(&self.0.body);

            m.mimic_case(interpolated).into()
        } else {
            let action = self.0.mimic_case_action(m.get_match());

            self.0.handle_mimic_action(action)
        }
    }
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
}
