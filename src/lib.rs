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
//! [`Accent`] is an array of [`Pass`] named blocks. Each block consists of rules. Regexes inside
//! [`Pass`] are compiled into a single big regex which allows walking haystack only once but
//! sometimes is not enough. For this reason you can define multiple separate ones.
//!
//! This library defines a rule layout that should make it easy to define speech accent. It does
//! the following:
//!
//! * defines `rules` - a list of passes
//! * allows adding or replacing existing rules for higher intensities of accent
//! * executes rules for matching severity group sequentially from top to bottom
//!
//! Intensities is a way to make accent worse. You can override certain rules or replace everything
//! completely.
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
//! * [`Template`] enables templating for inner tag
//! * [`NoTemplate`] disables templating for inner tag
//! * [`MimicCase`] enables case mimicking for inner tag
//! * [`NoMimicCase`] disables case mimicking for inner tag
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
//!     Match,
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
//!     fn generate<'a>(&self, m: &Match<'a>) -> std::borrow::Cow<'a, str> {
//!         if self.0 {
//!             m.get_match().to_uppercase()
//!         } else {
//!             m.get_match().to_lowercase()
//!         }.into()
//!     }
//! }
//!
//! // construct accent that will uppercase all instances of "a" and lowercase all "b"
//! let accent = ron::from_str::<Accent>(
//!     r#"
//! (
//!     accent: [
//!         (
//!             name: "patterns",
//!             rules: {
//!                 "a": {"StringCase": true},
//!                 "b": {"StringCase": false},
//!             }
//!         ),
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
//! `deserialize` | enables deserialization for [`Tag`] trait | yes
//! `cli` | required to run CLI tool | no
//!
//! [`Tag`]: crate::tag::Tag
//! [`Pass`]: crate::pass::Pass
//! [`Match`]: crate::Match
//! [`Intensity`]: crate::intensity::Intensity
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
mod pass;

// pub for bench
#[doc(hidden)]
pub mod utils;

#[cfg(feature = "deserialize")]
mod deserialize;

pub mod tag;
pub use accent::Accent;
pub use intensity::Intensity;
pub use pass::{Match, Pass};
