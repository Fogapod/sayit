use std::{borrow::Cow, error::Error, fmt};

use regex_automata::{
    meta::{BuildError, Regex},
    util::syntax,
};

use crate::{tag::Tag, Match};

/// A group of rules with their regexes combined into one
#[derive(Clone)]
pub struct Pass {
    regexes: Vec<String>,
    tags: Vec<Box<dyn Tag>>,
    multi_regex: Regex,
}

// skips 20 pages of debug output of `multi_regex` field
#[allow(clippy::missing_fields_in_debug)]
impl fmt::Debug for Pass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pass")
            .field("patterns", &self.regexes)
            .field("tags", &self.tags)
            .finish()
    }
}

impl Pass {
    /// Creates new instance from vec of regex and tag pairs
    #[allow(clippy::result_large_err)]
    pub fn new(rules: Vec<(String, Box<dyn Tag>)>) -> Result<Self, CreationError> {
        let (patterns, tags): (Vec<_>, Vec<_>) = rules.into_iter().unzip();

        let multi_regex = Regex::builder()
            .syntax(
                syntax::Config::new()
                    .multi_line(true)
                    .case_insensitive(true),
            )
            .build_many(&patterns)
            .map_err(CreationError::BadRegex)?;

        Ok(Self {
            regexes: patterns,
            multi_regex,
            tags,
        })
    }

    /// Merges it's own regexes with other. Tags for existing regexes are replaced while new ones
    /// are placed at the end of resulting new Pass
    #[allow(clippy::result_large_err)]
    pub fn extend(&self, other: Pass) -> Result<Self, CreationError> {
        let mut existing_rules: Vec<_> = self
            .regexes
            .iter()
            .cloned()
            .zip(self.tags.clone())
            .collect();

        let mut appended_rules = Vec::new();

        'outer: for (new_regex, new_tag) in other.regexes.into_iter().zip(other.tags.into_iter()) {
            for (existing_regex, existing_tag) in &mut existing_rules {
                if new_regex == **existing_regex {
                    *existing_tag = new_tag;
                    continue 'outer;
                }
            }

            appended_rules.push((new_regex, new_tag));
        }

        existing_rules.extend(appended_rules);

        Self::new(existing_rules)
    }

    /// Produces string with all non-overlapping regexes replaced by corresponding tags
    #[must_use]
    pub fn apply<'a>(&self, text: Cow<'a, str>) -> Cow<'a, str> {
        let input = text.as_ref();

        let mut caps_iter = self.multi_regex.captures_iter(input);

        let Some(mut captures) = caps_iter.next() else {
            return text;
        };

        let mut last_replacement = 0;
        let mut output = String::with_capacity(text.len());

        loop {
            // SAFETY: these captures come from matches. The only way this can fail is if they were
            //         created manually with Captures::empty()
            let caps_match = unsafe { captures.get_match().unwrap_unchecked() };

            let range = caps_match.range();
            let tag = &self.tags[caps_match.pattern()];

            let repl = tag.generate(&Match { input, captures });

            output.push_str(&text[last_replacement..range.start]);
            output.push_str(&repl);

            last_replacement = range.end;

            captures = match caps_iter.next() {
                Some(caps) => caps,
                None => break,
            };
        }

        output.push_str(&text[last_replacement..]);

        Cow::Owned(output)
    }
}

#[derive(Debug)]
pub enum CreationError {
    BadRegex(BuildError),
}

impl fmt::Display for CreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreationError::BadRegex(err) => {
                let mut msg = err.to_string();
                if let Some(syntax_msg) = err.syntax_error() {
                    msg = format!("msg: {syntax_msg}");
                }

                write!(f, "regex combination failed: {msg}")
            }
        }
    }
}

impl Error for CreationError {}

#[cfg(test)]
mod tests {
    use crate::tag_impls::Literal;

    use super::Pass;

    impl PartialEq for Pass {
        fn eq(&self, other: &Self) -> bool {
            self.regexes == other.regexes && self.tags == other.tags
        }
    }

    #[test]
    fn rules_replaced() {
        let old = Pass::new(vec![
            ("old".to_string(), Literal::new_boxed("old")),
            ("old2".to_string(), Literal::new_boxed("old2")),
        ])
        .unwrap();

        let new = Pass::new(vec![("old".to_string(), Literal::new_boxed("new"))]).unwrap();

        let extended = old.extend(new).unwrap();
        let expected = Pass::new(vec![
            ("old".to_string(), Literal::new_boxed("new")),
            ("old2".to_string(), Literal::new_boxed("old2")),
        ])
        .unwrap();

        assert_eq!(extended, expected);
    }

    #[test]
    fn rules_appended() {
        let old = Pass::new(vec![("existing".to_string(), Literal::new_boxed("old"))]).unwrap();
        let new = Pass::new(vec![("added".to_string(), Literal::new_boxed("new"))]).unwrap();

        let extended = old.extend(new).unwrap();
        let expected = Pass::new(vec![
            ("existing".to_string(), Literal::new_boxed("old")),
            ("added".to_string(), Literal::new_boxed("new")),
        ])
        .unwrap();

        assert_eq!(extended, expected);
    }
}
