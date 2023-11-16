use std::borrow::Cow;

use crate::utils::SimpleString;

use rand::seq::SliceRandom;
use regex::{Captures, Regex};

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct AnyReplacement(pub(crate) Vec<Replacement>);

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct WeightedReplacement(pub(crate) Vec<(u64, Replacement)>);

/// Receives match and provides replacement
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub(crate) enum Replacement {
    /// Do not replace
    Original,
    // TODO: either a separate variant or modify Simple to allow formatting using capture groups:
    //       "hello {1}" would insert group 1
    // TODO: implement a bit of serde magic for easier parsing: string would turn into `Simple`,
    //       array into `Any` and map with u64 keys into `Weights`
    /// Puts string as is
    Simple(SimpleString),
    /// Selects random replacement with equal weights
    Any(AnyReplacement),
    /// Selects replacement based on relative weights
    Weights(WeightedReplacement),
    // Uppercases inner replacement
    Uppercase(Box<Replacement>),
    // Lowercases inner replacement
    Lowercase(Box<Replacement>),
    // TODO: custom Fn or trait
    // #[serde(skip)]
    // Custom(Box<dyn CustomReplacement>),
}

impl Replacement {
    // FIXME: i dont know why these are marked as dead code, these are used in tests a lot. these
    //        should be made public eventually to allow programmatic accent construction like in
    //        tests
    #[allow(dead_code)]
    /// Construct new Original variant
    pub(crate) fn new_original() -> Self {
        Self::Original
    }

    #[allow(dead_code)]
    /// Construct new Simple variant
    pub(crate) fn new_simple(s: &str) -> Self {
        Self::Simple(SimpleString::new(s))
    }

    #[allow(dead_code)]
    /// Construct new Any variant
    pub(crate) fn new_any(items: Vec<Replacement>) -> Self {
        Self::Any(AnyReplacement(items))
    }

    #[allow(dead_code)]
    /// Construct new Weights variant
    pub(crate) fn new_weights(items: Vec<(u64, Replacement)>) -> Self {
        Self::Weights(WeightedReplacement(items))
    }

    #[allow(dead_code)]
    /// Construct new Uppercase variant
    pub(crate) fn new_uppercase(inner: Replacement) -> Self {
        Self::Uppercase(Box::new(inner))
    }

    #[allow(dead_code)]
    /// Construct new Lowercase variant
    pub(crate) fn new_lowercase(inner: Replacement) -> Self {
        Self::Lowercase(Box::new(inner))
    }

    fn apply<'a>(&self, caps: &Captures, input: &'a str, normalize_case: bool) -> Cow<'a, str> {
        // FIXME: use caps directly instead of indexing input. this is a bug of regex lifetimes:
        //        https://github.com/rust-lang/regex/discussions/775
        match self {
            Self::Original => {
                Cow::from(&input[caps.get(0).expect("match 0 is always present").range()])
            }
            Self::Simple(string) => {
                // TODO: currently templated strings do not match original case. this might be
                //       desired. the problem is that mimic_ascii_case relies on expensive
                //       precomputed fields inside SimpleString which would need to be recreated
                if string.has_template {
                    let mut dst = String::new();

                    caps.expand(&string.body, &mut dst);

                    dst.into()
                } else {
                    Cow::from(if normalize_case {
                        string.mimic_ascii_case(
                            &input[caps.get(0).expect("match 0 is always present").range()],
                        )
                    } else {
                        string.body.clone()
                    })
                }
            }
            Self::Any(AnyReplacement(items)) => {
                let mut rng = rand::thread_rng();

                items
                    .choose(&mut rng)
                    .expect("empty Any")
                    .apply(caps, input, normalize_case)
            }
            Self::Weights(WeightedReplacement(items)) => {
                let mut rng = rand::thread_rng();

                items
                    .choose_weighted(&mut rng, |item| item.0)
                    .expect("empty Weights")
                    .1
                    .apply(caps, input, normalize_case)
            }
            Replacement::Uppercase(inner) => {
                Cow::Owned(inner.apply(caps, input, false).to_uppercase())
            }
            Replacement::Lowercase(inner) => {
                Cow::Owned(inner.apply(caps, input, false).to_lowercase())
            }
        }
    }
}

/// Maps regex to replacement
#[derive(Debug)]
pub(crate) struct Rule {
    pub(crate) source: Regex,
    pub(crate) replacement: Replacement,
}

impl Rule {
    pub(crate) fn apply<'input>(
        &self,
        text: &'input str,
        normalize_case: bool,
    ) -> Cow<'input, str> {
        self.source.replace_all(text, |caps: &Captures| {
            self.replacement.apply(caps, text, normalize_case)
        })
    }
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.source.as_str() == other.source.as_str() && self.replacement == other.replacement
    }
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

    fn apply<'a>(
        replacement: &Replacement,
        self_matching_pattern: &'a str,
        normalize_case: bool,
    ) -> Cow<'a, str> {
        Cow::from(replacement.apply(
            &make_captions(self_matching_pattern),
            self_matching_pattern,
            normalize_case,
        ))
    }

    #[test]
    fn original() {
        let replacement = Replacement::new_original();

        assert_eq!(apply(&replacement, "bar", false), "bar");
        assert_eq!(apply(&replacement, "foo", false), "foo");
    }

    #[test]
    fn simple() {
        let replacement = Replacement::new_simple("bar");

        assert_eq!(apply(&replacement, "foo", false), "bar");
        assert_eq!(apply(&replacement, "bar", false), "bar");
    }

    #[test]
    fn any() {
        let replacement = Replacement::new_any(vec![
            Replacement::new_simple("bar"),
            Replacement::new_simple("baz"),
        ]);

        let selected = apply(&replacement, "bar", false).into_owned();

        assert!(["bar", "baz"].contains(&selected.as_str()));
    }

    #[test]
    fn weights() {
        let replacement = Replacement::new_weights(vec![
            (1, Replacement::new_simple("bar")),
            (1, Replacement::new_simple("baz")),
            (0, Replacement::new_simple("spam")),
        ]);

        let selected = apply(&replacement, "bar", false).into_owned();

        assert!(vec!["bar", "baz"].contains(&selected.as_str()));
    }

    #[test]
    fn uppercase() {
        let replacement = Replacement::new_uppercase(Replacement::new_original());

        assert_eq!(apply(&replacement, "lowercase", false), "LOWERCASE");
        assert_eq!(apply(&replacement, "UPPERCASE", false), "UPPERCASE");
        assert_eq!(apply(&replacement, "MiXeDcAsE", false), "MIXEDCASE");
        assert_eq!(apply(&replacement, "юникод", false), "ЮНИКОД");
    }

    #[test]
    fn lowercase() {
        let replacement = Replacement::new_lowercase(Replacement::new_original());

        assert_eq!(apply(&replacement, "lowercase", false), "lowercase");
        assert_eq!(apply(&replacement, "UPPERCASE", false), "uppercase");
        assert_eq!(apply(&replacement, "MiXeDcAsE", false), "mixedcase");
        assert_eq!(apply(&replacement, "ЮНИКОД", false), "юникод");
    }

    #[test]
    fn expansion() {
        let two_words_regex = Regex::new(r"(\w+) (\w+)").unwrap();

        let swap_words_replacement = Replacement::new_simple("$2 $1");
        assert_eq!(
            swap_words_replacement.apply(
                &two_words_regex.captures("swap us").unwrap(),
                "swap us",
                false
            ),
            "us swap"
        );

        // nonexistent goup results in empty string
        let delete_word_replacement = Replacement::new_simple("$nonexistent $2");
        assert_eq!(
            delete_word_replacement.apply(
                &two_words_regex.captures("DELETE US").unwrap(),
                "DELETE US",
                false
            ),
            " US"
        );
    }
}
