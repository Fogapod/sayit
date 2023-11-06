use regex::Regex;

#[cfg(feature = "deserialize")]
use crate::deserialize::SeverityBodyDef;
use crate::replacement::ReplacementCallback;

#[derive(Debug)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
#[cfg_attr(feature = "deserialize", serde(from = "SeverityBodyDef"))]
pub(crate) struct SeverityBody {
    pub(crate) words: Vec<(Regex, ReplacementCallback)>,
    pub(crate) patterns: Vec<(Regex, ReplacementCallback)>,
}

/// Either replaces everything from previous severity using `Replace` or adds new words and
/// patterns to the end of previous ones with `Extend`
#[derive(Debug)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub(crate) enum Severity {
    Replace(SeverityBody),
    Extend(SeverityBody),
}
