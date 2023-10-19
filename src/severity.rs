use crate::replacement::ReplacementCallback;

#[derive(Debug)]
pub(crate) struct SeverityBody {
    pub(crate) words: Vec<(String, ReplacementCallback)>,
    pub(crate) patterns: Vec<(String, ReplacementCallback)>,
}

/// Either replaces everything from previous severity using `Replace` or adds new words and
/// patterns to the end of previous ones with `Extend`
#[derive(Debug)]
pub(crate) enum Severity {
    Replace(SeverityBody),
    Extend(SeverityBody),
}
