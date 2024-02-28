use std::ops::Range;

use regex_automata::util::captures::Captures;

use crate::utils::{to_title_case, LiteralString, MimicAction};

pub(crate) struct LazyLiteral {
    body: String,
    length_hint: usize,
}

impl LazyLiteral {
    pub(crate) fn new(body: String, length_hint: usize) -> Self {
        Self { body, length_hint }
    }

    pub(crate) fn handle_mimic_action(self, action: MimicAction) -> String {
        match action {
            MimicAction::Title => to_title_case(&self.body),
            MimicAction::Uppercase => self.body.to_uppercase(),
            MimicAction::Nothing => self.body,
        }
    }
}

// slightly faster version than the one in utils, does not count total characters
fn count_cases(string: &str) -> (usize, usize) {
    string.chars().fold((0, 0), |(lower, upper), c| {
        let is_lower = c.is_lowercase();
        let is_upper = c.is_uppercase();

        (lower + usize::from(is_lower), upper + usize::from(is_upper))
    })
}

impl LiteralString for LazyLiteral {
    fn chars(&self) -> (usize, bool, bool) {
        let (lowercase, uppercase) = count_cases(&self.body);

        (self.length_hint, lowercase != 0, uppercase != 0)
    }
}

/// Holds [`regex_automata::util::captures::Captures`] and full input
#[derive(Debug)]
pub struct Match<'a> {
    pub(crate) captures: Captures,
    pub(crate) input: &'a str,
}

impl<'a> Match<'a> {
    /// # Safety
    ///
    /// Constructing with invalid Captures will cause UB in [`Match::get_range`] and
    /// [`Match::get_match`]
    pub unsafe fn new(captures: Captures, input: &'a str) -> Self {
        Self { captures, input }
    }

    /// Returns full match range (regex group 0)
    #[inline]
    pub fn get_range(&self) -> Range<usize> {
        // SAFETY: Match is guaranteed to be created from valid Captures and input or via unsafe
        //         constructor
        unsafe { self.captures.get_match().unwrap_unchecked() }.range()
    }

    /// Returns full match (regex group 0)
    #[inline]
    pub fn get_match(&self) -> &'a str {
        // SAFETY: Match is guaranteed to be created from valid Captures and input or via unsafe
        //         constructor
        unsafe { self.input.get_unchecked(self.get_range()) }
    }

    pub fn get_captures(&self) -> &Captures {
        &self.captures
    }

    pub fn get_input(&self) -> &'a str {
        self.input
    }

    /// Uses regex interpolation syntax to use current match in template
    #[must_use]
    pub fn interpolate(&self, template: &str) -> String {
        let mut dst = String::new();

        self.captures
            .interpolate_string_into(self.input, template, &mut dst);

        dst
    }

    /// Tries to match string case for current match
    #[must_use]
    pub fn mimic_case(&self, template: String) -> String {
        let len = self.get_range().len();
        let literal = LazyLiteral::new(template, len);
        let action = literal.mimic_case_action(self.get_match());

        literal.handle_mimic_action(action)
    }
}
