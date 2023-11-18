use std::borrow::Cow;

use crate::utils::SimpleString;

use rand::seq::SliceRandom;
use regex::Captures;

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct AnyReplacement(pub(crate) Vec<Replacement>);

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct WeightedReplacement(pub(crate) Vec<(u64, Replacement)>);

/// Receives match and provides replacement
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Replacement(Box<InnerReplacement>);

impl Replacement {
    /// Construct new Original variant
    pub fn new_original() -> Self {
        InnerReplacement::Original.into()
    }

    /// Construct new Simple variant
    pub fn new_simple(s: &str) -> Self {
        InnerReplacement::Simple(SimpleString::from(s)).into()
    }

    /// Construct new Any variant
    pub fn new_any(items: Vec<Replacement>) -> Self {
        InnerReplacement::Any(AnyReplacement(items)).into()
    }

    /// Construct new Weights variant
    pub fn new_weights(items: Vec<(u64, Replacement)>) -> Self {
        InnerReplacement::Weights(WeightedReplacement(items)).into()
    }

    /// Construct new Upper variant
    pub fn new_upper(inner: Replacement) -> Self {
        InnerReplacement::Upper(Box::new(inner)).into()
    }

    /// Construct new Lower variant
    pub fn new_lower(inner: Replacement) -> Self {
        InnerReplacement::Lower(Box::new(inner)).into()
    }

    /// Construct new Template variant
    pub fn new_template(inner: Replacement) -> Self {
        InnerReplacement::Template(Box::new(inner)).into()
    }

    /// Construct new NoTemplate variant
    pub fn new_no_template(inner: Replacement) -> Self {
        InnerReplacement::NoTemplate(Box::new(inner)).into()
    }

    /// Construct new MimicCase variant
    pub fn new_mimic_case(inner: Replacement) -> Self {
        InnerReplacement::MimicCase(Box::new(inner)).into()
    }

    /// Construct new NoMimicCase variant
    pub fn new_no_mimic_case(inner: Replacement) -> Self {
        InnerReplacement::NoMimicCase(Box::new(inner)).into()
    }

    /// Construct new Concat variant
    pub fn new_concat(left: Replacement, right: Replacement) -> Self {
        InnerReplacement::Concat(Box::new(left), Box::new(right)).into()
    }

    /// Applies replacement to given text. This is a mini version of Accent
    pub fn apply<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut mimic_case: Option<bool>,
        mut template: Option<bool>,
    ) -> Cow<'a, str> {
        // FIXME: use caps directly instead of indexing input. this is a bug of regex lifetimes:
        //        https://github.com/rust-lang/regex/discussions/775
        let mut replaced = match self.0.as_ref() {
            InnerReplacement::Original => {
                (&input[caps.get(0).expect("match 0 is always present").range()]).into()
            }
            InnerReplacement::Simple(string) => {
                template = template.or(Some(string.has_template));

                if mimic_case.unwrap_or(true) {
                    // prevent more expensive mimic_case from running after us
                    mimic_case = Some(false);

                    string.mimic_ascii_case(
                        &input[caps.get(0).expect("match 0 is always present").range()],
                    )
                } else {
                    string.body.clone()
                }
                .into()
            }
            InnerReplacement::Any(AnyReplacement(items)) => {
                let mut rng = rand::thread_rng();

                items
                    .choose(&mut rng)
                    .expect("empty Any")
                    .apply(caps, input, mimic_case, template)
            }
            InnerReplacement::Weights(WeightedReplacement(items)) => {
                let mut rng = rand::thread_rng();

                items
                    .choose_weighted(&mut rng, |item| item.0)
                    .expect("empty Weights")
                    .1
                    .apply(caps, input, mimic_case, template)
            }
            InnerReplacement::Upper(inner) => inner
                .apply(caps, input, Some(false), template)
                .to_uppercase()
                .into(),
            InnerReplacement::Lower(inner) => inner
                .apply(caps, input, Some(false), template)
                .to_lowercase()
                .into(),
            InnerReplacement::Template(inner) => inner.apply(caps, input, mimic_case, Some(true)),
            InnerReplacement::NoTemplate(inner) => {
                inner.apply(caps, input, mimic_case, Some(false))
            }
            InnerReplacement::MimicCase(inner) => inner.apply(caps, input, Some(true), template),
            InnerReplacement::NoMimicCase(inner) => inner.apply(caps, input, Some(false), template),
            InnerReplacement::Concat(left, right) => {
                left.apply(caps, input, mimic_case, template)
                    + right.apply(caps, input, mimic_case, template)
            }
        };

        if template.unwrap_or(false) {
            let mut dst = String::new();
            caps.expand(&replaced, &mut dst);
            replaced = dst.into();
        }

        if mimic_case.unwrap_or(false) {
            // FIXME: initializing SimpleString is super expensive!!!!
            let s = SimpleString::from(replaced.as_ref());
            replaced = s.mimic_ascii_case(input).into();
        }

        replaced
    }
}

impl From<InnerReplacement> for Replacement {
    fn from(inner: InnerReplacement) -> Self {
        Self(Box::new(inner))
    }
}

// This is private because it leaks internal types. Internal types are needed because current
// deserialization is extremely janky. If those are ever removed, it could be exposed directly
//
// TODO: implement a bit of serde magic for easier parsing: string would turn into `Simple`, array
//       into `Any` and map with u64 keys into `Weights`
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub(crate) enum InnerReplacement {
    /// Do not replace
    Original,

    /// Puts string as is
    Simple(SimpleString),

    /// Selects random replacement with equal weights
    Any(AnyReplacement),

    // TODO: implement a bit of serde magic for easier parsing: string would turn into `Simple`,
    //       array into `Any` and map with u64 keys into `Weights`
    /// Selects replacement based on relative weights
    Weights(WeightedReplacement),

    /// Uppercases inner replacement
    Upper(Box<Replacement>),

    /// Lowercases inner replacement
    Lower(Box<Replacement>),

    /// Enables $ templating using regex for inner
    Template(Box<Replacement>),

    /// Disables $ templating using regex for inner
    NoTemplate(Box<Replacement>),

    /// Enables string case mimicing for inner
    MimicCase(Box<Replacement>),

    /// Disables string case mimicing for inner
    NoMimicCase(Box<Replacement>),

    /// Joins left and right parts
    Concat(Box<Replacement>, Box<Replacement>),
    // TODO: custom Fn or trait
    // #[serde(skip)]
    // Custom(Box<dyn CustomReplacement>),
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use regex::{Captures, Regex};

    use super::Replacement;

    fn make_captions(self_matching_pattern: &str) -> Captures<'_> {
        Regex::new(self_matching_pattern)
            .unwrap()
            .captures(self_matching_pattern)
            .unwrap()
    }

    fn apply<'a>(replacement: &Replacement, self_matching_pattern: &'a str) -> Cow<'a, str> {
        replacement
            .apply(
                &make_captions(self_matching_pattern),
                self_matching_pattern,
                None,
                None,
            )
            .into()
    }

    #[test]
    fn original() {
        let replacement = Replacement::new_original();

        assert_eq!(apply(&replacement, "bar"), "bar");
        assert_eq!(apply(&replacement, "foo"), "foo");
    }

    #[test]
    fn simple() {
        let replacement = Replacement::new_simple("bar");

        assert_eq!(apply(&replacement, "foo"), "bar");
        assert_eq!(apply(&replacement, "bar"), "bar");
    }

    #[test]
    fn simple_templates_by_default() {
        let replacement = Replacement::new_simple("$0");

        assert_eq!(apply(&replacement, "foo"), "foo");
    }

    #[test]
    fn simple_mimics_case_by_default() {
        let replacement = Replacement::new_simple("bar");

        assert_eq!(apply(&replacement, "FOO"), "BAR");
    }

    #[test]
    fn constructed_not_templates_by_default() {
        let replacement =
            Replacement::new_concat(Replacement::new_simple("$"), Replacement::new_simple("0"));

        assert_eq!(apply(&replacement, "FOO"), "$0");
    }

    #[test]
    fn constructed_not_mimics_by_default() {
        let replacement = Replacement::new_concat(
            Replacement::new_no_mimic_case(Replacement::new_simple("b")),
            Replacement::new_no_mimic_case(Replacement::new_simple("ar")),
        );

        assert_eq!(apply(&replacement, "FOO"), "bar");
    }

    #[test]
    fn any() {
        let replacement = Replacement::new_any(vec![
            Replacement::new_simple("bar"),
            Replacement::new_simple("baz"),
        ]);

        let selected = apply(&replacement, "bar").into_owned();

        assert!(["bar", "baz"].contains(&selected.as_str()));
    }

    #[test]
    fn weights() {
        let replacement = Replacement::new_weights(vec![
            (1, Replacement::new_simple("bar")),
            (1, Replacement::new_simple("baz")),
            (0, Replacement::new_simple("spam")),
        ]);

        let selected = apply(&replacement, "bar").into_owned();

        assert!(vec!["bar", "baz"].contains(&selected.as_str()));
    }

    #[test]
    fn uppercase() {
        let replacement = Replacement::new_upper(Replacement::new_original());

        assert_eq!(apply(&replacement, "lowercase"), "LOWERCASE");
        assert_eq!(apply(&replacement, "UPPERCASE"), "UPPERCASE");
        assert_eq!(apply(&replacement, "MiXeDcAsE"), "MIXEDCASE");
        assert_eq!(apply(&replacement, "юникод"), "ЮНИКОД");
    }

    #[test]
    fn lowercase() {
        let replacement = Replacement::new_lower(Replacement::new_original());

        assert_eq!(apply(&replacement, "lowercase"), "lowercase");
        assert_eq!(apply(&replacement, "UPPERCASE"), "uppercase");
        assert_eq!(apply(&replacement, "MiXeDcAsE"), "mixedcase");
        assert_eq!(apply(&replacement, "ЮНИКОД"), "юникод");
    }

    #[test]
    fn concat() {
        let replacement =
            Replacement::new_concat(Replacement::new_original(), Replacement::new_original());

        assert_eq!(apply(&replacement, "double"), "doubledouble");
    }

    #[test]
    fn template() {
        let replacement =
            Replacement::new_no_template(Replacement::new_template(Replacement::new_simple("$0")));

        assert_eq!(apply(&replacement, "template"), "template");

        let replacement = Replacement::new_template(Replacement::new_concat(
            Replacement::new_simple("$"),
            Replacement::new_simple("0"),
        ));
        assert_eq!(apply(&replacement, "template"), "template");
    }

    #[test]
    fn no_template() {
        let replacement = Replacement::new_no_template(Replacement::new_simple("$0"));

        assert_eq!(apply(&replacement, "template"), "$0");
    }

    #[test]
    fn mimic_case() {
        let replacement = Replacement::new_no_mimic_case(Replacement::new_mimic_case(
            Replacement::new_simple("bar"),
        ));

        assert_eq!(apply(&replacement, "FOO"), "BAR");

        let replacement = Replacement::new_mimic_case(Replacement::new_concat(
            Replacement::new_simple("b"),
            Replacement::new_simple("ar"),
        ));
        assert_eq!(apply(&replacement, "FOO"), "BAR");
    }

    #[test]
    fn no_mimic_case() {
        let replacement = Replacement::new_no_mimic_case(Replacement::new_simple("bar"));

        assert_eq!(apply(&replacement, "FOO"), "bar");
    }

    #[test]
    fn expansion() {
        let two_words_regex = Regex::new(r"(\w+) (\w+)").unwrap();

        let swap_words_replacement = Replacement::new_simple("$2 $1");
        assert_eq!(
            swap_words_replacement.apply(
                &two_words_regex.captures("swap us").unwrap(),
                "swap us",
                None,
                None
            ),
            "us swap"
        );

        // nonexistent goup results in empty string
        let delete_word_replacement = Replacement::new_simple("$nonexistent $2");
        assert_eq!(
            delete_word_replacement.apply(
                &two_words_regex.captures("DELETE US").unwrap(),
                "DELETE US",
                None,
                None
            ),
            " US"
        );
    }
}
