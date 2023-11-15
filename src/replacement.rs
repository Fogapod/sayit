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
    // TODO: see below
    // Custom(fn taking Caps, severity and maybe other info),
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

    fn apply(&self, caps: &Captures, normalize_case: bool) -> String {
        match self {
            Self::Original => caps[0].to_owned(),
            Self::Simple(string) => {
                if normalize_case {
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
                    .apply(caps, normalize_case)
            }
            Self::Weights(WeightedReplacement(items)) => {
                let mut rng = rand::thread_rng();

                items
                    .choose_weighted(&mut rng, |item| item.0)
                    .expect("empty Weights")
                    .1
                    .apply(caps, normalize_case)
            }
            Replacement::Uppercase(inner) => {
                inner.apply(caps, false).to_uppercase()
            }
            Replacement::Lowercase(inner) => {
                inner.apply(caps, false).to_lowercase()
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
    pub(crate) fn apply(&self, text: &str, normalize_case: bool) -> String {
        self.source
            .replace_all(text, |caps: &Captures| self.replacement.apply(caps, normalize_case))
            .into_owned()
    }
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.source.as_str() == other.source.as_str() && self.replacement == other.replacement
    }
}

#[cfg(test)]
mod tests {
    use regex::{Captures, Regex};

    use super::Replacement;

    fn make_captions(self_matching_pattern: &str) -> Captures<'_> {
        Regex::new(self_matching_pattern)
            .unwrap()
            .captures(self_matching_pattern)
            .unwrap()
    }

    #[test]
    fn original() {
        let replacement = Replacement::new_original();

        let bar_capture = make_captions("bar");
        let foo_capture = make_captions("foo");

        assert_eq!(replacement.apply(&bar_capture, false), "bar".to_owned());
        assert_eq!(replacement.apply(&foo_capture, false), "foo".to_owned());
    }

    #[test]
    fn simple() {
        let replacement = Replacement::new_simple("bar");

        let bar_capture = Regex::new("bar").unwrap().captures("bar").unwrap();
        let foo_capture = Regex::new("foo").unwrap().captures("foo").unwrap();

        assert_eq!(replacement.apply(&bar_capture, false), "bar".to_owned());
        assert_eq!(replacement.apply(&foo_capture, false), "bar".to_owned());
    }

    #[test]
    fn any() {
        let replacement = Replacement::new_any(vec![
            Replacement::new_simple("bar"),
            Replacement::new_simple("baz"),
        ]);

        let bar_capture = Regex::new("bar").unwrap().captures("bar").unwrap();
        let selected = replacement.apply(&bar_capture, false);

        assert!(vec!["bar".to_owned(), "baz".to_owned()].contains(&selected));
    }

    #[test]
    fn weights() {
        let replacement = Replacement::new_weights(vec![
            (1, Replacement::new_simple("bar")),
            (1, Replacement::new_simple("baz")),
            (0, Replacement::new_simple("spam")),
        ]);

        let bar_capture = Regex::new("bar").unwrap().captures("bar").unwrap();
        let selected = replacement.apply(&bar_capture, false);

        assert!(vec!["bar".to_owned(), "baz".to_owned()].contains(&selected));
    }

    #[test]
    fn uppercase() {
        let uppercase = Replacement::new_uppercase(Replacement::new_original());

        assert_eq!(
            uppercase.apply(&make_captions("lowercase"), false),
            "LOWERCASE".to_owned()
        );
        assert_eq!(
            uppercase.apply(&make_captions("UPPERCASE"), false),
            "UPPERCASE".to_owned()
        );
        assert_eq!(
            uppercase.apply(&make_captions("MiXeDcAsE"), false),
            "MIXEDCASE".to_owned()
        );
        assert_eq!(
            uppercase.apply(&make_captions("юникод"), false),
            "ЮНИКОД".to_owned()
        );
    }
    #[test]
    fn lowercase() {
        let lowercase = Replacement::new_lowercase(Replacement::new_original());

        assert_eq!(
            lowercase.apply(&make_captions("lowercase"), false),
            "lowercase".to_owned()
        );
        assert_eq!(
            lowercase.apply(&make_captions("UPPERCASE"), false),
            "uppercase".to_owned()
        );
        assert_eq!(
            lowercase.apply(&make_captions("MiXeDcAsE"), false),
            "mixedcase".to_owned()
        );
        assert_eq!(
            lowercase.apply(&make_captions("ЮНИКОД"), false),
            "юникод".to_owned()
        );
    }
}
