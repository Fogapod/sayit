use regex::Regex;

#[cfg(feature = "deserialize")]
use crate::deserialize::IntensityBodyDef;
use crate::replacement::Replacement;

#[derive(Debug)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
#[cfg_attr(feature = "deserialize", serde(from = "IntensityBodyDef"))]
pub(crate) struct IntensityBody {
    pub(crate) words: Vec<(Regex, Replacement)>,
    pub(crate) patterns: Vec<(Regex, Replacement)>,
}

/// Either replaces everything from previous intensity using `Replace` or adds new words and
/// patterns to the end of previous ones with `Extend`
#[derive(Debug)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub(crate) enum Intensity {
    Replace(IntensityBody),
    Extend(IntensityBody),
}
