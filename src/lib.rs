//! String replacements using regex.
//!
//! # Table of contents
//! * [Description](#description)
//! * [Accent](#accent)
//! * [Tag trait](#tag-trait)
//! * [Implementing Tag trait](#implementing-tag-trait)
//! * [CLI tool](#cli-tool)
//! * [Feature flags](#feature-flags)
//!
//! # Description
//!
//! Provides a way to define a set of rules for replacing text in string. Each rule consists of
//! regex pattern and Tag trait object. The original use case is to simulate
//! mispronounciations in speech accents via text.
//!
//! # Accent
//!
//! [`Accent`] is a collection of rules that define speech accent. Rule is a pair of regex and tag.
//! Deserialization is currently the only way to create it.
//!
//! This library defines a rule layout that should make it easy to define speech accent. It does
//! the following:
//!
//! * defines `patterns` - a list of pairs regex -> tag
//! * defines `words` - same as `patterns` except regexes are automatically surrounded by `\b`
//! * allows adding or replacing existing rules for higher intensities of accent
//! * executes rules for matching severity group sequentially from top to bottom
//!
//! See `examples` folder, specifically `example.ron` for full reference.
//!
//! # Tag trait
//!
//! [`Tag`] is a core of this library. It is an extendable trait. After implementing it you
//! can can parse it along default implementations and other ones.
//!
//! Default replacments are:
//!
//! * [`Original`] does not replace (leaves original match as is)
//! * [`Literal`] puts given string
//! * [`Any`] selects random inner tag with equal weights
//! * [`Weights`] selects random inner tag based on relative weights
//! * [`Upper`] converts inner result to uppercase
//! * [`Lower`] converts inner result to lowercase
//! * [`Template`] enables templating for inner type
//! * [`NoTemplate`] disables templating for inner type
//! * [`MimicCase`] enables case mimicking for inner type
//! * [`NoMimicCase`] disables case mimicking for inner type
//! * [`Concat`] runs left and right inner tags and adds them together
//!
//! # Implementing Tag trait
//!
//! `StringCase` either uppercases or lowercases match depending of given boolean:
//!
//! ```rust
//! use sayit::{
//!     tag::Tag,
//!     Accent,
//! };
//!
//! // Deserialize is only required with `deserialize` crate feature
//! #[derive(Clone, Debug, serde::Deserialize)]
//! // transparent allows using `true` directly instead of `(true)`
//! #[serde(transparent)]
//! pub struct StringCase(bool);
//!
//! // `typetag` is only required with `deserialize` crate feature
//! #[typetag::deserialize]
//! impl Tag for StringCase {
//!     // 'a is source text lifetime
//!     fn generate<'a>(
//!         &self,
//!         // Regex capture
//!         caps: &regex::Captures,
//!         // Entire source text. This is required because of regex crate lifetime limitation:
//!         // <https://github.com/rust-lang/regex/issues/777>. It will be removed after regex 2.0
//!         input: &'a str,
//!     ) -> std::borrow::Cow<'a, str> {
//!         if self.0 {
//!             self.current_match(caps, input).to_uppercase()
//!         } else {
//!             self.current_match(caps, input).to_lowercase()
//!         }.into()
//!     }
//! }
//!
//! // construct accent that will uppercase all instances of "a" and lowercase all "b"
//! let accent = ron::from_str::<Accent>(
//!     r#"
//! (
//!     patterns: {
//!         "a": {"StringCase": true},
//!         "b": {"StringCase": false},
//!     }
//! )
//! "#,
//! )
//! .expect("accent did not parse");
//!
//! assert_eq!(accent.say_it("abab ABAB Hello", 0), "AbAb AbAb Hello");
//! ```
//!
//! # CLI tool
//!
//! You can run CLI tool by enabling `cli` feature: `cargo run --features=cli -- --help`.  
//! It is useful for debugging and can run any of the example accents.
//!
//! `echo hello | cargo run --features=cli -- -a examples/clown.ron` will honk at you.
//!
//! # Feature flags
//!
//! Name | Description | Default?
//! ---|---|---
//! `deserialize` | enables deserialization for [`Tag`] trait | yes
//! `cli` | required to run CLI tool | no
//!
//! [`Tag`]: crate::tag::Tag
//! [`Original`]: crate::tag::Original
//! [`Literal`]: crate::tag::Literal
//! [`Any`]: crate::tag::Any
//! [`Weights`]: crate::tag::Weights
//! [`Upper`]: crate::tag::Upper
//! [`Lower`]: crate::tag::Lower
//! [`Template`]: crate::tag::Template
//! [`NoTemplate`]: crate::tag::NoTemplate
//! [`MimicCase`]: crate::tag::MimicCase
//! [`NoMimicCase`]: crate::tag::NoMimicCase
//! [`Concat`]: crate::tag::Concat

mod accent;
mod intensity;
mod rule;

// pub for bench
#[doc(hidden)]
pub mod utils;

#[cfg(feature = "deserialize")]
mod deserialize;

pub mod tag;
pub use accent::Accent;
