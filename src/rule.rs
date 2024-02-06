use std::borrow::Cow;

use regex::{Captures, Regex};

use crate::tag::{Tag, TagOptions};

/// Maps regex to tag
#[derive(Debug)]
pub(crate) struct Rule {
    pub(crate) source: Regex,
    pub(crate) tag: Box<dyn Tag>,
}

impl Rule {
    pub(crate) fn apply<'input>(&self, text: &'input str) -> Cow<'input, str> {
        self.source.replace_all(text, |caps: &Captures| {
            self.tag.apply_options(caps, text, TagOptions::default())
        })
    }
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.source.as_str() == other.source.as_str()
    }
}
