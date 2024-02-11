use crate::utils::LiteralString;

use std::{borrow::Cow, fmt::Debug};

use dyn_clone::{clone_trait_object, DynClone};
use rand::seq::SliceRandom;
use regex::Captures;

/// Alters behaviour of some tags
#[derive(Default)]
pub struct TagOptions {
    template: Option<bool>,
    mimic_case: Option<bool>,
}

impl TagOptions {
    /// Default expensive case mimicking implementation
    pub fn mimic_case(target: &str, source: &str) -> String {
        let s = LiteralString::from(target);
        s.mimic_ascii_case(source)
    }

    /// Default templating implementation using [`regex::Captures::expand`]
    pub fn template(template: &str, caps: &Captures) -> String {
        let mut dst = String::new();
        caps.expand(template, &mut dst);
        dst
    }
}

/// Receives match and provides replacement
#[cfg_attr(feature = "deserialize", typetag::deserialize)]
pub trait Tag: DynClone + Debug {
    /// Select suitable replacement
    ///
    /// caps is actual match
    /// input is reference to full input
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str>;

    /// Takes [`generate`][Self::generate] output and applies additional operations set by options
    ///
    /// Redefine this if you want to mess with options for inner tags or have some big optimization
    /// for options
    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        options: TagOptions,
    ) -> Cow<'a, str> {
        let template = options.template.unwrap_or(false);
        let mimic_case = options.mimic_case.unwrap_or(false);

        let generated = self.generate(caps, input);

        match (template, mimic_case) {
            (false, false) => generated,
            (true, false) => TagOptions::template(&generated, caps).into(),
            (false, true) => {
                TagOptions::mimic_case(&generated, self.current_match(caps, input)).into()
            }
            (true, true) => {
                let templated = TagOptions::template(&generated, caps);
                TagOptions::mimic_case(&templated, self.current_match(caps, input)).into()
            }
        }
    }

    /// Returns full match (group 0)
    fn current_match<'a>(&self, caps: &Captures, input: &'a str) -> &'a str {
        &input[caps.get(0).expect("match 0 is always present").range()]
    }
}

clone_trait_object!(Tag);

/// Same as [`Literal`] with `"$0"` argument: returns entire match.
///
/// Does not act as template by default unlike [`Literal`]
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
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        self.current_match(caps, input).into()
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
    fn generate<'a>(&self, _: &Captures, _: &'a str) -> Cow<'a, str> {
        self.0.body.clone().into()
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        options: TagOptions,
    ) -> Cow<'a, str> {
        let template = options.template.unwrap_or(self.0.has_template);
        let mimic_case = options.mimic_case.unwrap_or(true);

        match (template, mimic_case) {
            (false, false) => self.generate(caps, input),
            (true, false) => TagOptions::template(&self.0.body, caps).into(),
            (false, true) => self
                .0
                .mimic_ascii_case(self.current_match(caps, input))
                .into(),
            (true, true) => {
                let templated = TagOptions::template(&self.0.body, caps);
                TagOptions::mimic_case(&templated, self.current_match(caps, input)).into()
            }
        }
    }
}

/// [`Any`] creation might fail
#[derive(Debug)]
pub enum AnyError {
    /// Must provide at least one element
    ZeroItems,
}

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
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        let mut rng = rand::thread_rng();

        self.0
            .choose(&mut rng)
            .expect("empty Any")
            .generate(caps, input)
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

/// Selects any of nested items with relative probabilities
#[derive(Clone, Debug)]
pub struct Weights(Vec<(u64, Box<dyn Tag>)>);

impl Weights {
    pub fn new(items: Vec<(u64, Box<dyn Tag>)>) -> Result<Self, WeightsError> {
        if items.is_empty() {
            return Err(WeightsError::ZeroItems);
        }
        if items.iter().fold(0, |sum, (w, _)| sum + w) == 0 {
            return Err(WeightsError::NonPositiveTotalWeights);
        }

        Ok(Self(items))
    }

    pub fn new_boxed(items: Vec<(u64, Box<dyn Tag>)>) -> Result<Box<Self>, WeightsError> {
        Ok(Box::new(Self::new(items)?))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Weights {
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        let mut rng = rand::thread_rng();

        self.0
            .choose_weighted(&mut rng, |item| item.0)
            .expect("empty Weights")
            .1
            .generate(caps, input)
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
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        self.0.generate(caps, input)
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: TagOptions,
    ) -> Cow<'a, str> {
        let template = options.template.take().unwrap_or(false);
        // do not mimic case inside this because it will be overwritten
        let _ = options.mimic_case.insert(false);

        let mut generated = self.0.apply_options(caps, input, options);

        if template {
            generated = TagOptions::template(&generated, caps).into();
        }

        generated.to_uppercase().into()
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
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        self.0.generate(caps, input)
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: TagOptions,
    ) -> Cow<'a, str> {
        let template = options.template.take().unwrap_or(false);
        // do not mimic case inside this because it will be overwritten
        let _ = options.mimic_case.insert(false);

        let mut generated = self.0.apply_options(caps, input, options);

        if template {
            generated = TagOptions::template(&generated, caps).into();
        }

        generated.to_lowercase().into()
    }
}

/// Enables templating for inner tag
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Template(Box<dyn Tag>);

impl Template {
    pub fn new(inner: Box<dyn Tag>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Tag>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for Template {
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        self.0.generate(caps, input)
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: TagOptions,
    ) -> Cow<'a, str> {
        options.template.get_or_insert(true);

        self.0.apply_options(caps, input, options)
    }
}

/// Disables templating for inner tag
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct NoTemplate(Box<dyn Tag>);

impl NoTemplate {
    pub fn new(inner: Box<dyn Tag>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Tag>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for NoTemplate {
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        self.0.generate(caps, input)
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: TagOptions,
    ) -> Cow<'a, str> {
        options.template.get_or_insert(false);

        self.0.apply_options(caps, input, options)
    }
}

/// Enables case mimicking for inner tag
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct MimicCase(Box<dyn Tag>);

impl MimicCase {
    pub fn new(inner: Box<dyn Tag>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Tag>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for MimicCase {
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        self.0.generate(caps, input)
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: TagOptions,
    ) -> Cow<'a, str> {
        options.mimic_case.get_or_insert(true);

        self.0.apply_options(caps, input, options)
    }
}

/// Disables case mimicking for inner tag
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct NoMimicCase(Box<dyn Tag>);

impl NoMimicCase {
    pub fn new(inner: Box<dyn Tag>) -> Self {
        Self(inner)
    }

    pub fn new_boxed(inner: Box<dyn Tag>) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[cfg_attr(feature = "deserialize", typetag::deserialize)]
impl Tag for NoMimicCase {
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        self.0.generate(caps, input)
    }

    fn apply_options<'a>(
        &self,
        caps: &Captures,
        input: &'a str,
        mut options: TagOptions,
    ) -> Cow<'a, str> {
        options.mimic_case.get_or_insert(false);

        self.0.apply_options(caps, input, options)
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
    fn generate<'a>(&self, caps: &Captures, input: &'a str) -> Cow<'a, str> {
        self.0 .0.generate(caps, input) + self.0 .1.generate(caps, input)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use regex::{Captures, Regex};

    use crate::tag::{
        Any, Concat, Literal, Lower, MimicCase, NoMimicCase, NoTemplate, Original, Tag, TagOptions,
        Template, Upper, Weights,
    };

    fn make_captions(pattern: &str) -> Captures<'_> {
        Regex::new(".+").unwrap().captures(pattern).unwrap()
    }

    fn apply<'a>(tag: &dyn Tag, self_matching_pattern: &'a str) -> Cow<'a, str> {
        tag.apply_options(
            &make_captions(self_matching_pattern),
            self_matching_pattern,
            TagOptions::default(),
        )
        .into()
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
    fn literal_templates_by_default() {
        let tag = Literal::new("$0");

        assert_eq!(apply(&tag, "foo"), "foo");
    }

    #[test]
    fn literal_mimics_case_by_default() {
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
    fn concatenated_not_templates_by_default() {
        let tag = Concat::new_boxed(Literal::new_boxed("$"), Literal::new_boxed("0"));

        assert_eq!(apply(tag.as_ref(), "FOO"), "$0");
        assert_eq!(apply(&Template::new(tag), "FOO"), "FOO");
    }

    #[test]
    fn concatenated_not_mimics_by_default() {
        let tag = Concat::new_boxed(
            NoMimicCase::new_boxed(Literal::new_boxed("b")),
            NoMimicCase::new_boxed(Literal::new_boxed("ar")),
        );

        assert_eq!(apply(tag.as_ref(), "FOO"), "bar");
        assert_eq!(apply(&MimicCase::new(tag), "FOO"), "BAR");
    }

    #[test]
    fn template() {
        // template without case mimicking
        let tag = NoMimicCase::new(Literal::new_boxed("$0"));

        assert_eq!(apply(&tag, "template"), "template");

        // template on then off
        let tag = NoTemplate::new(Template::new_boxed(Literal::new_boxed("$0")));

        assert_eq!(apply(&tag, "template"), "$0");

        // constructed template
        let tag = Template::new(Concat::new_boxed(
            Literal::new_boxed("$"),
            Literal::new_boxed("0"),
        ));
        assert_eq!(apply(&tag, "template"), "template");

        // template for Original
        let tag = Template::new(Original::new_boxed());
        assert_eq!(apply(&tag, "$0 long"), "$0 long long");
    }

    #[test]
    fn no_template() {
        let tag = NoTemplate::new(Literal::new_boxed("$0"));

        assert_eq!(apply(&tag, "template"), "$0");
    }

    #[test]
    fn mimic_case() {
        let tag = NoMimicCase::new(MimicCase::new_boxed(Literal::new_boxed("bar")));

        assert_eq!(apply(&tag, "FOO"), "bar");

        let tag = MimicCase::new(Concat::new_boxed(
            Literal::new_boxed("b"),
            Literal::new_boxed("ar"),
        ));
        assert_eq!(apply(&tag, "FOO"), "BAR");
    }

    #[test]
    fn no_mimic_case() {
        let tag = NoMimicCase::new(Literal::new_boxed("bar"));

        assert_eq!(apply(&tag, "FOO"), "bar");
    }

    #[test]
    fn template_and_mimic_case() {
        let tag = MimicCase::new(Template::new_boxed(Concat::new_boxed(
            Literal::new_boxed(""),
            Lower::new_boxed(Literal::new_boxed("$0")),
        )));

        assert_eq!(apply(&tag, "TEMPLATE"), "TEMPLATE");
    }

    #[test]
    fn mimic_case_propagates_through_everything() {
        let tag = MimicCase::new(NoMimicCase::new_boxed(Concat::new_boxed(
            Any::new_boxed(vec![Literal::new_boxed("bar")]).unwrap(),
            Weights::new_boxed(vec![(1, Literal::new_boxed("test"))]).unwrap(),
        )));

        assert_eq!(apply(&tag, "FOO"), "BARTEST");
    }

    #[test]
    fn template_propagates_through_everything() {
        let tag = NoTemplate::new(Template::new_boxed(Concat::new_boxed(
            Any::new_boxed(vec![Literal::new_boxed("$0")]).unwrap(),
            Weights::new_boxed(vec![(1, Literal::new_boxed("$0"))]).unwrap(),
        )));

        assert_eq!(apply(&tag, "1"), "$0$0");
    }

    #[test]
    fn expansion() {
        let two_words_regex = Regex::new(r"(\w+) (\w+)").unwrap();

        let swap_words_tag = Literal::new("$2 $1");
        assert_eq!(
            swap_words_tag.apply_options(
                &two_words_regex.captures("swap us").unwrap(),
                "swap us",
                TagOptions::default()
            ),
            "us swap"
        );

        // nonexistent goup results in empty string
        let delete_word_tag = Literal::new("$nonexistent $2");
        assert_eq!(
            delete_word_tag.apply_options(
                &two_words_regex.captures("DELETE US").unwrap(),
                "DELETE US",
                TagOptions::default()
            ),
            " US"
        );
    }
}
