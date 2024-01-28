//! String replacements using regex.
//!
//! # Table of contents
//! * [Description](#description)
//! * [Accent](#accent)
//! * [Replacement trait](#replacement-trait)
//! * [Implementing Replacement trait](#implementing-replacement-trait)
//! * [CLI tool](#cli-tool)
//! * [Feature flags](#feature-flags)
//!
//! # Description
//!
//! Provides a way to define a set of rules for replacing text in string. Each rule consists of
//! regex pattern and Replacement trait object. The original use case is to simulate
//! mispronounciations in speech accents via text.
//!
//! # Accent
//!
//! [`Accent`] is a collection of rules that define speech accent. Rule is a pair of regex and
//! replacement. Deserialization is currently the only way to create it.
//!
//! This library defines a rule layout that should make it easy to define speech accent. It does
//! the following:
//!
//! * defines `patterns` - a list of pairs regex -> replacement
//! * defines `words` - same as `patterns` except regexes are automatically surrounded by `\b`
//! * allows adding or replacing existing rules for higher intensities of accent
//! * executes rules for matching severity group sequentially from top to bottom
//!
//! See `examples` folder, specifically `example.ron` for full reference.
//!
//! # Replacement trait
//!
//! [`Replacement`] is a core of this library. It is an extendable trait. After implementing it you
//! can can parse it along default implementations and other ones.
//!
//! Default replacments are:
//!
//! * [`replacement::Original`] does not replace (leaves original match as is)
//! * [`replacement::Literal`] puts given string
//! * [`replacement::Any`] selects random inner replacement with equal weights
//! * [`replacement::Weights`] selects random inner replacement based on relative weights
//! * [`replacement::Upper`] converts inner result to uppercase
//! * [`replacement::Lower`] converts inner result to lowercase
//! * [`replacement::Template`] enables templating for inner type
//! * [`replacement::NoTemplate`] disables templating for inner type
//! * [`replacement::MimicCase`] enables case mimicking for inner type
//! * [`replacement::NoMimicCase`] disables case mimicking for inner type
//! * [`replacement::Concat`] runs left and right inner branches and adds them together
//!
//! # Implementing Replacement trait
//!
//! `StringCase` either uppercases or lowercases match depending of given boolean:
//!
//! ```rust
//! use sayit::{
//!     replacement::{Replacement, ReplacementOptions},
//!     Accent,
//! };
//!
//! // Deserialize is only required with `deserialize` crate feature
//! #[derive(Clone, Debug, serde::Deserialize)]
//! pub struct StringCase(bool);
//!
//! // `typetag` is only required with `deserialize` crate feature
//! #[typetag::deserialize]
//! impl Replacement for StringCase {
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
//!     patterns: [
//!         ("a", {"StringCase": (true)}),
//!         ("b", {"StringCase": (false)}),
//!     ]
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
//! `deserialize` | enables deserialization for [`Replacement`] trait | yes
//! `ron` | enables ron format deserialization, requires `deserialize` | no
//! `cli` | required to run CLI tool | no
//!
//! [`Replacement`]: crate::replacement::Replacement

mod accent;
mod intensity;
mod rule;

// pub for bench
#[doc(hidden)]
pub mod utils;

#[cfg(feature = "deserialize")]
mod deserialize;

pub mod replacement;
pub use accent::Accent;
