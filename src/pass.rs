use std::{borrow::Cow, fmt};

use regex_automata::{meta::Regex, util::syntax};

use crate::{tag::Tag, Match};

/// A group of rules with their regexes combined into one
#[derive(Clone)]
pub struct Pass {
    pub(crate) name: String,
    regexes: Vec<String>,
    tags: Vec<Box<dyn Tag>>,
    multi_regex: Regex,
}

// skips 20 pages of debug output of `multi_regex` field
impl fmt::Debug for Pass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pass")
            .field("name", &self.name)
            .field("patterns", &self.regexes)
            .field("tags", &self.tags)
            .finish()
    }
}

impl Pass {
    pub fn new<T: AsRef<str>>(name: &str, rules: Vec<(T, Box<dyn Tag>)>) -> Result<Self, String> {
        let (patterns, tags): (Vec<_>, Vec<_>) = rules.into_iter().unzip();

        let patterns: Vec<_> = patterns
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect();

        let multi_regex = Regex::builder()
            .syntax(
                syntax::Config::new()
                    .multi_line(true)
                    .case_insensitive(true),
            )
            .build_many(&patterns)
            .map_err(|err| format!("regex combination failed: {err}"))?;

        Ok(Self {
            name: name.to_owned(),
            regexes: patterns,
            multi_regex,
            tags,
        })
    }

    pub fn extend(&self, other: Pass) -> Result<Self, String> {
        let mut existing_rules: Vec<_> = self
            .regexes
            .iter()
            .cloned()
            .zip(self.tags.iter().cloned())
            .collect();

        let mut appended_rules = Vec::new();

        for (new_regex, new_tag) in other.regexes.into_iter().zip(other.tags.into_iter()) {
            let mut replaced = false;

            for (existing_regex, existing_tag) in &mut existing_rules {
                if &new_regex == existing_regex {
                    // FIXME: remove clone
                    *existing_tag = new_tag.clone();
                    replaced = true;
                    break;
                }
            }

            if !replaced {
                appended_rules.push((new_regex, new_tag));
            }
        }

        existing_rules.extend(appended_rules);

        Self::new(&other.name, existing_rules)
    }

    pub fn apply<'a>(&self, text: &'a str) -> Cow<'a, str> {
        let all_captures: Vec<_> = self.multi_regex.captures_iter(text).collect();

        if all_captures.is_empty() {
            return Cow::Borrowed(text);
        }

        let mut last_replacement = 0;
        let mut output = String::with_capacity(text.len());

        for caps in all_captures {
            let caps_match = caps.get_match().expect("this matched");
            let range = caps_match.range();
            let tag = &self.tags[caps_match.pattern()];

            let repl = tag.generate(&Match {
                captures: caps,
                input: text,
            });

            output.extend([&text[last_replacement..range.start], &repl]);

            last_replacement = range.end;
        }

        output.push_str(&text[last_replacement..]);

        Cow::Owned(output)
    }
}

#[cfg(test)]
mod tests {
    use crate::{pass::Pass, tag_impls::Literal};

    impl PartialEq for Pass {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name && self.regexes == other.regexes && self.tags == other.tags
        }
    }

    #[test]
    fn rules_replaced() {
        let old = Pass::new(
            "",
            vec![
                ("old", Literal::new_boxed("old")),
                ("old2", Literal::new_boxed("old2")),
            ],
        )
        .unwrap();

        let new = Pass::new("", vec![("old", Literal::new_boxed("new"))]).unwrap();

        let extended = old.extend(new).unwrap();
        let expected = Pass::new(
            "",
            vec![
                ("old", Literal::new_boxed("new")),
                ("old2", Literal::new_boxed("old2")),
            ],
        )
        .unwrap();

        assert_eq!(extended, expected);
    }

    #[test]
    fn rules_appended() {
        let old = Pass::new("", vec![("existing", Literal::new_boxed("old"))]).unwrap();
        let new = Pass::new("", vec![("added", Literal::new_boxed("new"))]).unwrap();

        let extended = old.extend(new).unwrap();
        let expected = Pass::new(
            "",
            vec![
                ("existing", Literal::new_boxed("old")),
                ("added", Literal::new_boxed("new")),
            ],
        )
        .unwrap();

        assert_eq!(extended, expected);
    }
}
