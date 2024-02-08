#[cfg(feature = "deserialize")]
use crate::deserialize::IntensityBodyDef;

use crate::tag::Tag;

use regex::Regex;

#[derive(Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(from = "IntensityBodyDef")
)]
pub(crate) struct IntensityBody {
    pub(crate) words: Vec<(Regex, Box<dyn Tag>)>,
    pub(crate) patterns: Vec<(Regex, Box<dyn Tag>)>,
}

/// Either replaces everything from previous intensity using `Replace` or adds new words and
/// patterns to the end of previous ones with `Extend`
#[derive(Debug)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub(crate) enum Intensity {
    Replace(IntensityBody),
    Extend(IntensityBody),
}
