use crate::replacement::ReplacementCallback;

#[derive(Debug)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub(crate) struct SeverityBody {
    #[cfg_attr(feature = "deserialize", serde(default))]
    pub(crate) words: Vec<(String, ReplacementCallback)>,
    #[cfg_attr(feature = "deserialize", serde(default))]
    pub(crate) patterns: Vec<(String, ReplacementCallback)>,
}

/// Either replaces everything from previous severity using `Replace` or adds new words and
/// patterns to the end of previous ones with `Extend`
#[derive(Debug)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
pub(crate) enum Severity {
    Replace(SeverityBody),
    Extend(SeverityBody),
}
