use crate::utils::SimpleString;

use rand::seq::SliceRandom;
use regex::{Captures, Regex};

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct AnyReplacement(pub(crate) Vec<ReplacementCallback>);

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct WeightedReplacement(pub(crate) Vec<(u64, ReplacementCallback)>);

/// Receives match and provides replacement
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub(crate) enum ReplacementCallback {
    /// Do not replace
    Noop,
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
    // TODO: see below
    // Custom(fn taking Caps, severity and maybe other info),
}

impl ReplacementCallback {
    // FIXME: i dont know why these are marked as dead code, these are used in tests a lot. these
    //        should be made public eventually to allow programmatic accent construction like in
    //        tests
    #[allow(dead_code)]
    /// Construct new Noop variant
    pub(crate) fn new_noop() -> Self {
        Self::Noop
    }

    #[allow(dead_code)]
    /// Construct new Simple variant
    pub(crate) fn new_simple(s: &str) -> Self {
        Self::Simple(SimpleString::new(s))
    }

    #[allow(dead_code)]
    /// Construct new Any variant
    pub(crate) fn new_any(items: Vec<ReplacementCallback>) -> Self {
        Self::Any(AnyReplacement(items))
    }

    #[allow(dead_code)]
    /// Construct new Weights variant
    pub(crate) fn new_weights(items: Vec<(u64, ReplacementCallback)>) -> Self {
        Self::Weights(WeightedReplacement(items))
    }

    fn apply(&self, caps: &Captures, normalize_case_: bool) -> String {
        match self {
            Self::Noop => caps[0].to_owned(),
            Self::Simple(string) => {
                if normalize_case_ {
                    string.mimic_ascii_case(&caps[0])
                } else {
                    string.body.clone()
                }
            }
            Self::Any(AnyReplacement(items)) => {
                let mut rng = rand::thread_rng();

                items
                    .choose(&mut rng)
                    .expect("empty Any")
                    .apply(caps, normalize_case_)
            }
            Self::Weights(WeightedReplacement(items)) => {
                let mut rng = rand::thread_rng();

                items
                    .choose_weighted(&mut rng, |item| item.0)
                    .expect("empty Weights")
                    .1
                    .apply(caps, normalize_case_)
            }
        }
    }
}

/// Maps regex to callback
#[derive(Debug)]
pub(crate) struct Replacement {
    pub(crate) source: Regex,
    pub(crate) cb: ReplacementCallback,
}

impl Replacement {
    pub(crate) fn apply(&self, text: &str, normalize_case: bool) -> String {
        self.source
            .replace_all(text, |caps: &Captures| self.cb.apply(caps, normalize_case))
            .into_owned()
    }
}

impl PartialEq for Replacement {
    fn eq(&self, other: &Self) -> bool {
        self.source.as_str() == other.source.as_str() && self.cb == other.cb
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::ReplacementCallback;

    #[test]
    fn callback_none() {
        let replacement = ReplacementCallback::new_noop();

        let bar_capture = Regex::new("bar").unwrap().captures("bar").unwrap();
        let foo_capture = Regex::new("foo").unwrap().captures("foo").unwrap();

        assert_eq!(replacement.apply(&bar_capture, false), "bar".to_owned());
        assert_eq!(replacement.apply(&foo_capture, false), "foo".to_owned());
    }

    #[test]
    fn callback_simple() {
        let replacement = ReplacementCallback::new_simple("bar");

        let bar_capture = Regex::new("bar").unwrap().captures("bar").unwrap();
        let foo_capture = Regex::new("foo").unwrap().captures("foo").unwrap();

        assert_eq!(replacement.apply(&bar_capture, false), "bar".to_owned());
        assert_eq!(replacement.apply(&foo_capture, false), "bar".to_owned());
    }

    #[test]
    fn callback_any() {
        let replacement = ReplacementCallback::new_any(vec![
            ReplacementCallback::new_simple("bar"),
            ReplacementCallback::new_simple("baz"),
        ]);

        let bar_capture = Regex::new("bar").unwrap().captures("bar").unwrap();
        let selected = replacement.apply(&bar_capture, false);

        assert!(vec!["bar".to_owned(), "baz".to_owned()].contains(&selected));
    }

    #[test]
    fn callback_weights() {
        let replacement = ReplacementCallback::new_weights(vec![
            (1, ReplacementCallback::new_simple("bar")),
            (1, ReplacementCallback::new_simple("baz")),
            (0, ReplacementCallback::new_simple("spam")),
        ]);

        let bar_capture = Regex::new("bar").unwrap().captures("bar").unwrap();
        let selected = replacement.apply(&bar_capture, false);

        assert!(vec!["bar".to_owned(), "baz".to_owned()].contains(&selected));
    }
}
