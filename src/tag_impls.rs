#[cfg(feature = "deserialize")]
use crate::deserialize::SortedMap;

use std::{borrow::Cow, error::Error, fmt::Display};

use crate::{tag::Tag, utils::LiteralString, Match};

/// Same as [`Literal`] with `"$0"` argument: returns entire match.
///
/// Does not act as template unlike [`Literal`]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub struct Original;

#[allow(clippy::new_without_default)]
impl Original {
    pub fn new() -> Self {
        Self {}
    }

    pub fn new_boxed() -> Box<Self> {
        Box::new(Self::new())
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Original {
    fn generate<'a>(&self, m: &Match<'a>) -> Cow<'a, str> {
        m.get_match().into()
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
pub struct Literal(LiteralString);

impl Literal {
    pub fn new(s: &str) -> Self {
        Self(LiteralString::from(s))
    }

    pub fn new_boxed(s: &str) -> Box<Self> {
        Box::new(Self::new(s))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Literal {
    fn generate<'a>(&self, m: &Match<'a>) -> Cow<'a, str> {
        if self.0.has_template {
            let interpolated = m.interpolate(&self.0.body);

            m.mimic_ascii_case(&interpolated)
        } else {
            self.0.mimic_ascii_case(m.get_match())
        }
        .into()
    }
}

/// [`Any`] creation might fail
#[derive(Debug)]
pub enum AnyError {
    /// Must provide at least one element
    ZeroItems,
}

impl Display for AnyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ZeroItems => "expected at least one element to choose from",
            }
        )
    }
}

impl Error for AnyError {}

/// Selects any of nested items with equal probabilities
#[derive(Clone, Debug)]
pub struct Any(Vec<Box<dyn Tag>>);

impl Any {
    pub fn new(items: Vec<Box<dyn Tag>>) -> Result<Self, AnyError> {
        if items.is_empty() {
            return Err(AnyError::ZeroItems);
        }

        Ok(Self(items))
    }

    pub fn new_boxed(items: Vec<Box<dyn Tag>>) -> Result<Box<Self>, AnyError> {
        Ok(Box::new(Self::new(items)?))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Any {
    fn generate<'a>(&self, m: &Match<'a>) -> Cow<'a, str> {
        let i = fastrand::usize(..self.0.len());

        self.0[i].generate(m)
    }
}

/// [`Weights`] creation might fail
#[derive(Debug)]
pub enum WeightsError {
    /// Must provide at least one element
    ZeroItems,
    /// Sum of all weights must be positive
    NonPositiveTotalWeights,
}

impl Display for WeightsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ZeroItems => "expected at least one element to choose from",
                Self::NonPositiveTotalWeights => "weights must add up to a positive number",
            }
        )
    }
}

/// Selects any of nested items with relative probabilities
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(try_from = "SortedMap<u64, Box<dyn Tag>, false>")
)]
pub struct Weights {
    choices: Vec<Box<dyn Tag>>,
    cum_weights: Vec<u64>,
}

impl Weights {
    pub fn new(items: Vec<(u64, Box<dyn Tag>)>) -> Result<Self, WeightsError> {
        let (weights, choices) = items.into_iter().unzip();

        let cum_weights = Self::cum_weights(weights)?;

        Ok(Self {
            choices,
            cum_weights,
        })
    }

    pub fn new_boxed(items: Vec<(u64, Box<dyn Tag>)>) -> Result<Box<Self>, WeightsError> {
        Ok(Box::new(Self::new(items)?))
    }

    fn cum_weights(mut weights: Vec<u64>) -> Result<Vec<u64>, WeightsError> {
        if weights.is_empty() {
            return Err(WeightsError::ZeroItems);
        }

        let mut previous = weights[0];
        for w in &mut weights[1..] {
            *w += previous;
            previous += *w - previous;
        }

        if weights[weights.len() - 1] == 0 {
            return Err(WeightsError::NonPositiveTotalWeights);
        }

        Ok(weights)
    }

    fn random_choice(&self) -> usize {
        let random_point = fastrand::u64(0..self.cum_weights.len() as u64);

        match self.cum_weights.binary_search(&random_point) {
            Ok(i) | Err(i) => i,
        }
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Weights {
    fn generate<'a>(&self, m: &Match<'a>) -> Cow<'a, str> {
        self.choices[self.random_choice()].generate(m)
    }
}

/// Uppercases result of inner tag
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Upper(Box<dyn Tag>);

impl Upper {
    pub fn new(inner: Box<dyn Tag>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Tag>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Upper {
    fn generate<'a>(&self, m: &Match<'a>) -> Cow<'a, str> {
        self.0.generate(m).to_uppercase().into()
    }
}

/// Lowercases result of inner tag
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Lower(Box<dyn Tag>);

impl Lower {
    pub fn new(inner: Box<dyn Tag>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Tag>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Lower {
    fn generate<'a>(&self, m: &Match<'a>) -> Cow<'a, str> {
        self.0.generate(m).to_lowercase().into()
    }
}

/// Adds results of left and right tags together
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Concat((Box<dyn Tag>, Box<dyn Tag>));

impl Concat {
    pub fn new(left: Box<dyn Tag>, right: Box<dyn Tag>) -> Self {
        Self((left, right))
    }

    pub fn new_boxed(left: Box<dyn Tag>, right: Box<dyn Tag>) -> Box<Self> {
        Box::new(Self::new(left, right))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Concat {
    fn generate<'a>(&self, m: &Match<'a>) -> Cow<'a, str> {
        self.0 .0.generate(m) + self.0 .1.generate(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Match;

    use std::borrow::Cow;

    use regex_automata::meta::Regex;

    fn make_match(pattern: &str) -> Match {
        let re = Regex::new(".+").unwrap();
        let mut caps = re.create_captures();

        re.captures(pattern, &mut caps);

        Match {
            captures: caps,
            input: pattern,
        }
    }

    fn apply<'a>(tag: &dyn Tag, self_matching_pattern: &'a str) -> Cow<'a, str> {
        tag.generate(&make_match(self_matching_pattern)).into()
    }

    #[test]
    fn original() {
        let tag = Original::new();

        assert_eq!(apply(&tag, "bar"), "bar");
        assert_eq!(apply(&tag, "foo"), "foo");
    }

    #[test]
    fn literal() {
        let tag = Literal::new("bar");

        assert_eq!(apply(&tag, "foo"), "bar");
        assert_eq!(apply(&tag, "bar"), "bar");
    }

    #[test]
    fn literal_templates() {
        let tag = Literal::new("$0");

        assert_eq!(apply(&tag, "foo"), "foo");
    }

    #[test]
    fn literal_mimics_case() {
        let tag = Literal::new("bar");

        assert_eq!(apply(&tag, "FOO"), "BAR");
    }

    #[test]
    fn any() {
        let tag = Any::new(vec![Literal::new_boxed("bar"), Literal::new_boxed("baz")]).unwrap();

        let selected = apply(&tag, "bar").into_owned();

        assert!(["bar", "baz"].contains(&selected.as_str()));
    }

    #[test]
    fn weights_cum_weights_errors() {
        assert!(Weights::cum_weights(Vec::new()).is_err());
        assert!(Weights::cum_weights(vec![0, 0, 0, 0, 0]).is_err());
    }

    #[test]
    fn weights_cum_weights() {
        assert_eq!(
            Weights::cum_weights(vec![1, 2, 3, 4, 5]).unwrap(),
            vec![1, 3, 6, 10, 15]
        );
        assert_eq!(
            Weights::cum_weights(vec![5, 4, 3, 2, 1]).unwrap(),
            vec![5, 9, 12, 14, 15]
        );
    }

    #[test]
    fn weights() {
        let tag = Weights::new(vec![
            (1, Literal::new_boxed("bar")),
            (1, Literal::new_boxed("baz")),
            (0, Literal::new_boxed("spam")),
        ])
        .unwrap();

        let selected = apply(&tag, "bar").into_owned();

        assert!(vec!["bar", "baz"].contains(&selected.as_str()));
    }

    #[test]
    fn weights_single() {
        let tag = Weights::new(vec![(50, Literal::new_boxed("test"))]).unwrap();

        let selected = apply(&tag, "test").into_owned();

        assert_eq!(selected, "test");
    }

    #[test]
    fn upper() {
        // double wrapped for coverage
        let tag = Upper::new(Upper::new_boxed(Original::new_boxed()));

        assert_eq!(apply(&tag, "lowercase"), "LOWERCASE");
        assert_eq!(apply(&tag, "UPPERCASE"), "UPPERCASE");
        assert_eq!(apply(&tag, "MiXeDcAsE"), "MIXEDCASE");
        assert_eq!(apply(&tag, "юникод"), "ЮНИКОД");
    }

    #[test]
    fn lower() {
        // double wrapped for coverage
        let tag = Lower::new(Lower::new_boxed(Original::new_boxed()));

        assert_eq!(apply(&tag, "lowercase"), "lowercase");
        assert_eq!(apply(&tag, "UPPERCASE"), "uppercase");
        assert_eq!(apply(&tag, "MiXeDcAsE"), "mixedcase");
        assert_eq!(apply(&tag, "ЮНИКОД"), "юникод");
    }

    #[test]
    fn concat() {
        let tag = Concat::new(Original::new_boxed(), Original::new_boxed());

        assert_eq!(apply(&tag, "double"), "doubledouble");
    }

    #[test]
    fn concatenated_not_mimics() {
        let tag = Concat::new(Literal::new_boxed("b"), Literal::new_boxed("ar"));

        assert_eq!(apply(&tag, "FOO"), "BAR");
    }

    #[test]
    fn expansion() {
        let swap_words_tag = Literal::new("$2 $1");

        let two_words_regex = Regex::new(r"(\w+) (\w+)").unwrap();
        let mut caps = two_words_regex.create_captures();
        two_words_regex.captures("swap us", &mut caps);

        assert_eq!(
            swap_words_tag.generate(&Match {
                captures: caps,
                input: "swap us",
            }),
            "us swap"
        );

        // nonexistent goup results in empty string
        let delete_word_tag = Literal::new("$nonexistent $2");

        let mut caps = two_words_regex.create_captures();
        two_words_regex.captures("DELETE US", &mut caps);

        assert_eq!(
            delete_word_tag.generate(&Match {
                captures: caps,
                input: "DELETE US",
            },),
            " US"
        );
    }
}
