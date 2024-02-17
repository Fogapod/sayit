use regex_automata::util::captures::Captures;

use crate::utils::LiteralString;

/// Holds [`regex_automata::util::captures::Captures`] and full input
#[derive(Debug)]
pub struct Match<'a> {
    pub captures: Captures,
    pub input: &'a str,
}

impl<'a> Match<'a> {
    /// Returns full match (regex group 0)
    pub fn get_match(&self) -> &'a str {
        &self.input[self.captures.get_match().expect("this matched").range()]
    }

    /// Uses regex interpolation syntax to use current match in template
    pub fn interpolate(&self, template: &str) -> String {
        let mut dst = String::new();

        self.captures
            .interpolate_string_into(self.input, template, &mut dst);

        dst
    }

    /// Tries to match string case for current match
    pub fn mimic_ascii_case(&self, template: &str) -> String {
        LiteralString::from(template).mimic_ascii_case(self.get_match())
    }
}
