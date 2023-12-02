use dyn_clone::clone_trait_object;
use dyn_clone::DynClone;
use std::borrow::Cow;
use std::fmt::Debug;

use crate::utils::LiteralString;

use rand::seq::SliceRandom;
use regex::Captures;

#[derive(Clone, Default)]
pub struct ReplacementOptions {
    template: Option<bool>,
    mimic_case: Option<bool>,
}

clone_trait_object!(Replacement);

/// Receives match and provides replacement
#[cfg_attr(feature = "deserialize", typetag::deserialize)]
pub trait Replacement: DynClone + Debug {
    /// Select suitable replacement
    fn generate<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        options: ReplacementOptions,
    ) -> Cow<'a, str>;

    /// Runs after `generate` and applies additional operations set by options
    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let template = options.template.take().unwrap_or(false);
        let mimic_case = options.mimic_case.take().unwrap_or(false);

        let mut transformed = self.generate(caps, input, options);

        if template {
            transformed = self.template(&transformed, caps).into()
        }

        if mimic_case {
            transformed = self.mimic_case(&transformed, input).into();
        }

        transformed
    }

    fn template(&self, template: &str, caps: &Captures) -> String {
        let mut dst = String::new();
        caps.expand(template, &mut dst);
        dst
    }

    fn mimic_case(&self, target: &str, source: &str) -> String {
        // FIXME: initializing LiteralString is super expensive!!!!
        let s = LiteralString::from(target);
        s.mimic_ascii_case(source)
    }
}

/// Shortcut for `"$0"` `Literal`. Returns entire match. Does not act as template by default unlike
/// `Literal`
#[derive(Clone, Debug, Default)] // i dont know why clippy DEMANDS default here. its nuts
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub struct Original;

impl Original {
    pub fn new() -> Self {
        Self {}
    }

    pub fn new_boxed() -> Box<Self> {
        Box::new(Self::new())
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for Original {
    fn generate<'a>(&self, caps: &Captures, input: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        (&input[caps.get(0).expect("match 0 is always present").range()]).into()
    }
}

/// Static string. Acts as template, see regex docs for syntax:
/// `https://docs.rs/regex/latest/regex/struct.Regex.html#example-9`
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
impl Replacement for Literal {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // implementation lives in `apply` entirely
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        options: ReplacementOptions,
    ) -> Cow<'a, str> {
        // ignore template flag if no template detected
        let template = options.template.unwrap_or(self.0.has_template);
        let mimic_case = options.mimic_case.unwrap_or(true);

        if !(template || mimic_case) {
            return self.0.body.clone().into();
        }

        if template && !mimic_case {
            return self.template(&self.0.body, caps).into();
        }

        if !template && mimic_case {
            return self
                .0
                .mimic_ascii_case(&input[caps.get(0).expect("match 0 is always present").range()])
                .into();
        }

        // expensive path because we cannot use precomputed LiteralString, new one must be created
        // after templating
        let templated = self.template(&self.0.body, caps);
        self.mimic_case(&templated, input).into()
    }
}

/// Any creation might fail
#[derive(Debug)]
pub enum AnyError {
    /// Must provide at least one element
    ZeroItems,
}

/// Selects any of nested items with equal probabilities
#[derive(Clone, Debug)]
pub struct Any(Vec<Box<dyn Replacement>>);

impl Any {
    pub fn new(items: Vec<Box<dyn Replacement>>) -> Result<Self, AnyError> {
        if items.is_empty() {
            return Err(AnyError::ZeroItems);
        }

        Ok(Self(items))
    }

    pub fn new_boxed(items: Vec<Box<dyn Replacement>>) -> Result<Box<Self>, AnyError> {
        Ok(Box::new(Self::new(items)?))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for Any {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // does not transform input so does not need to implement this
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let mut rng = rand::thread_rng();

        self.0
            .choose(&mut rng)
            .expect("empty Any")
            .apply_options(caps, input, options)
    }
}

/// Any creation might fail
#[derive(Debug)]
pub enum WeightsError {
    /// Must provide at least one element
    ZeroItems,
    /// Sum of all weights must be positive
    NonPositiveTotalWeights,
}

/// Selects any of nested items with relative probabilities
#[derive(Clone, Debug)]
pub struct Weights(Vec<(u64, Box<dyn Replacement>)>);

impl Weights {
    pub fn new(items: Vec<(u64, Box<dyn Replacement>)>) -> Result<Self, WeightsError> {
        if items.is_empty() {
            return Err(WeightsError::ZeroItems);
        }
        if items.iter().fold(0, |sum, (w, _)| sum + w) == 0 {
            return Err(WeightsError::NonPositiveTotalWeights);
        }

        Ok(Self(items))
    }

    pub fn new_boxed(items: Vec<(u64, Box<dyn Replacement>)>) -> Result<Box<Self>, WeightsError> {
        Ok(Box::new(Self::new(items)?))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for Weights {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // does not transform input so does not need to implement this
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let mut rng = rand::thread_rng();

        self.0
            .choose_weighted(&mut rng, |item| item.0)
            .expect("empty Weights")
            .1
            .apply_options(caps, input, options)
    }
}

/// Uppercases result of inner replacement
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Upper(Box<dyn Replacement>);

impl Upper {
    pub fn new(inner: Box<dyn Replacement>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Replacement>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for Upper {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // this does not do anything on its own, only changes case, invalidating mimic_case
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let template = options.template.take().unwrap_or(false);
        // do not mimic case inside this because it will be overwritten
        let _ = options.mimic_case.insert(false);

        let mut replaced = self.0.apply_options(caps, input, options);

        if template {
            replaced = self.template(&replaced, caps).into();
        }

        replaced.to_uppercase().into()
    }
}

/// Lowercases result of inner replacement
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Lower(Box<dyn Replacement>);

impl Lower {
    pub fn new(inner: Box<dyn Replacement>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Replacement>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for Lower {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // this does not do anything on its own, only changes case, invalidating mimic_case
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let template = options.template.take().unwrap_or(false);
        // do not mimic case inside this because it will be overwritten
        let _ = options.mimic_case.insert(false);

        let mut replaced = self.0.apply_options(caps, input, options);

        if template {
            replaced = self.template(&replaced, caps).into();
        }

        replaced.to_lowercase().into()
    }
}

/// Enables templating for inner replacement
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Template(Box<dyn Replacement>);

impl Template {
    pub fn new(inner: Box<dyn Replacement>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Replacement>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for Template {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // this only changes options
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let _ = options.template.insert(true);
        self.0.apply_options(caps, input, options)
    }
}

/// Disables templating for inner replacement
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct NoTemplate(Box<dyn Replacement>);

impl NoTemplate {
    pub fn new(inner: Box<dyn Replacement>) -> Self {
        Self(inner)
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for NoTemplate {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // this only changes options
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let _ = options.template.insert(false);
        self.0.apply_options(caps, input, options)
    }
}

/// Enables case mimicking for inner replacement
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct MimicCase(Box<dyn Replacement>);

impl MimicCase {
    pub fn new(inner: Box<dyn Replacement>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Replacement>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for MimicCase {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // this only changes options
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let _ = options.mimic_case.insert(true);
        self.0.apply_options(caps, input, options)
    }
}

/// Disables case mimicking for inner replacement
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct NoMimicCase(Box<dyn Replacement>);

impl NoMimicCase {
    pub fn new(inner: Box<dyn Replacement>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Replacement>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for NoMimicCase {
    fn generate<'a>(&self, _: &Captures, _: &'a str, _: ReplacementOptions) -> Cow<'a, str> {
        // this only changes options
        panic!("not meant to be called directly");
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: ReplacementOptions,
    ) -> Cow<'a, str> {
        let _ = options.mimic_case.insert(false);
        self.0.apply_options(caps, input, options)
    }
}

/// Adds results of left and right replacements together
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Concat((Box<dyn Replacement>, Box<dyn Replacement>));

impl Concat {
    pub fn new(left: Box<dyn Replacement>, right: Box<dyn Replacement>) -> Self {
        Self((left, right))
    }

    pub fn new_boxed(left: Box<dyn Replacement>, right: Box<dyn Replacement>) -> Box<Self> {
        Box::new(Self::new(left, right))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Replacement for Concat {
    fn generate<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        options: ReplacementOptions,
    ) -> Cow<'a, str> {
        self.0 .0.apply_options(caps, input, options.clone())
            + self.0 .1.apply_options(caps, input, options)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use regex::{Captures, Regex};

    use crate::replacement::{
        Any, Concat, Literal, Lower, MimicCase, NoMimicCase, NoTemplate, Original, Replacement,
        ReplacementOptions, Template, Upper, Weights,
    };

    fn make_captions(self_matching_pattern: &str) -> Captures<'_> {
        Regex::new(self_matching_pattern)
            .unwrap()
            .captures(self_matching_pattern)
            .unwrap()
    }

    fn apply<'a>(replacement: &dyn Replacement, self_matching_pattern: &'a str) -> Cow<'a, str> {
        replacement
            .apply_options(
                &make_captions(self_matching_pattern),
                self_matching_pattern,
                ReplacementOptions::default(),
            )
            .into()
    }

    #[test]
    fn original() {
        let replacement = Original::new();

        assert_eq!(apply(&replacement, "bar"), "bar");
        assert_eq!(apply(&replacement, "foo"), "foo");
    }

    #[test]
    fn literal() {
        let replacement = Literal::new("bar");

        assert_eq!(apply(&replacement, "foo"), "bar");
        assert_eq!(apply(&replacement, "bar"), "bar");
    }

    #[test]
    fn literal_templates_by_default() {
        let replacement = Literal::new("$0");

        assert_eq!(apply(&replacement, "foo"), "foo");
    }

    #[test]
    fn literal_mimics_case_by_default() {
        let replacement = Literal::new("bar");

        assert_eq!(apply(&replacement, "FOO"), "BAR");
    }

    #[test]
    fn constructed_not_templates_by_default() {
        let replacement = Concat::new(Literal::new_boxed("$"), Literal::new_boxed("0"));

        assert_eq!(apply(&replacement, "FOO"), "$0");
    }

    #[test]
    fn constructed_not_mimics_by_default() {
        let replacement = Concat::new(
            NoMimicCase::new_boxed(Literal::new_boxed("b")),
            NoMimicCase::new_boxed(Literal::new_boxed("ar")),
        );

        assert_eq!(apply(&replacement, "FOO"), "bar");
    }

    #[test]
    fn any() {
        let replacement =
            Any::new(vec![Literal::new_boxed("bar"), Literal::new_boxed("baz")]).unwrap();

        let selected = apply(&replacement, "bar").into_owned();

        assert!(["bar", "baz"].contains(&selected.as_str()));
    }

    #[test]
    fn weights() {
        let replacement = Weights::new(vec![
            (1, Literal::new_boxed("bar")),
            (1, Literal::new_boxed("baz")),
            (0, Literal::new_boxed("spam")),
        ])
        .unwrap();

        let selected = apply(&replacement, "bar").into_owned();

        assert!(vec!["bar", "baz"].contains(&selected.as_str()));
    }

    #[test]
    fn upper() {
        let replacement = Upper::new(Original::new_boxed());

        assert_eq!(apply(&replacement, "lowercase"), "LOWERCASE");
        assert_eq!(apply(&replacement, "UPPERCASE"), "UPPERCASE");
        assert_eq!(apply(&replacement, "MiXeDcAsE"), "MIXEDCASE");
        assert_eq!(apply(&replacement, "юникод"), "ЮНИКОД");
    }

    #[test]
    fn lower() {
        let replacement = Lower::new(Original::new_boxed());

        assert_eq!(apply(&replacement, "lowercase"), "lowercase");
        assert_eq!(apply(&replacement, "UPPERCASE"), "uppercase");
        assert_eq!(apply(&replacement, "MiXeDcAsE"), "mixedcase");
        assert_eq!(apply(&replacement, "ЮНИКОД"), "юникод");
    }

    #[test]
    fn concat() {
        let replacement = Concat::new(Original::new_boxed(), Original::new_boxed());

        assert_eq!(apply(&replacement, "double"), "doubledouble");
    }

    #[test]
    fn template() {
        let replacement = NoTemplate::new(Template::new_boxed(Literal::new_boxed("$0")));

        assert_eq!(apply(&replacement, "template"), "template");

        let replacement = Template::new(Concat::new_boxed(
            Literal::new_boxed("$"),
            Literal::new_boxed("0"),
        ));
        assert_eq!(apply(&replacement, "template"), "template");
    }

    #[test]
    fn no_template() {
        let replacement = NoTemplate::new(Literal::new_boxed("$0"));

        assert_eq!(apply(&replacement, "template"), "$0");
    }

    #[test]
    fn mimic_case() {
        let replacement = NoMimicCase::new(MimicCase::new_boxed(Literal::new_boxed("bar")));

        assert_eq!(apply(&replacement, "FOO"), "BAR");

        let replacement = MimicCase::new(Concat::new_boxed(
            Literal::new_boxed("b"),
            Literal::new_boxed("ar"),
        ));
        assert_eq!(apply(&replacement, "FOO"), "BAR");
    }

    #[test]
    fn no_mimic_case() {
        let replacement = NoMimicCase::new(Literal::new_boxed("bar"));

        assert_eq!(apply(&replacement, "FOO"), "bar");
    }

    #[test]
    fn expansion() {
        let two_words_regex = Regex::new(r"(\w+) (\w+)").unwrap();

        let swap_words_replacement = Literal::new("$2 $1");
        assert_eq!(
            swap_words_replacement.apply_options(
                &two_words_regex.captures("swap us").unwrap(),
                "swap us",
                ReplacementOptions::default()
            ),
            "us swap"
        );

        // nonexistent goup results in empty string
        let delete_word_replacement = Literal::new("$nonexistent $2");
        assert_eq!(
            delete_word_replacement.apply_options(
                &two_words_regex.captures("DELETE US").unwrap(),
                "DELETE US",
                ReplacementOptions::default()
            ),
            " US"
        );
    }
}
