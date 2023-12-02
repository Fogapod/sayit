use std::borrow::Cow;

use regex::{Captures, Regex};

use crate::replacement::{Replacement, ReplacementOptions};

/// Maps regex to replacement
#[derive(Debug)]
pub(crate) struct Rule {
    pub(crate) source: Regex,
    pub(crate) replacement: Box<dyn Replacement>,
}

impl Rule {
    pub(crate) fn apply<'input>(&self, text: &'input str) -> Cow<'input, str> {
        self.source.replace_all(text, |caps: &Captures| {
            self.replacement
                .apply_options(caps, text, ReplacementOptions::default())
        })
    }
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.source.as_str() == other.source.as_str()
    }
}
