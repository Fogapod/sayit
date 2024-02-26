use std::ops::Range;

use regex_automata::util::captures::Captures;

use crate::utils::{LazyLiteral, LiteralString};

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
