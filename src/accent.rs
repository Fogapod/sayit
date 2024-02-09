#[cfg(feature = "deserialize")]
use crate::deserialize::AccentDef;

use crate::{intensity::Intensity, rule::Rule, tag::Tag};

use std::{borrow::Cow, collections::BTreeMap};

use regex::Regex;

/// Replaces patterns in text according to rules
#[derive(Debug, PartialEq)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(try_from = "AccentDef")
)]
pub struct Accent {
    // a set of rules for each intensity level, sorted from lowest to highest
    pub(crate) intensities: Vec<(u64, Vec<Rule>)>,
}

impl Accent {
    fn merge_rules(first: &[(Regex, Box<dyn Tag>)], second: &[(Regex, Box<dyn Tag>)]) -> Vec<Rule> {
        first
            .iter()
            .chain(second)
            .map(|(regex, tag)| Rule {
                source: regex.clone(),
                tag: tag.clone(),
            })
            .collect()
    }

    // keeps collection order, rewrites left duplicates with right ones
    // TODO: investigate the usefulness of defining same pattern multiple times. Since rules are
    //       sequentional, are there situations when we might want to apply something on top of
    //       another change? `"*": Lower(Original); ...; Lower(Original)` this might be a hacky way
    //       to fix something in complex accents
    fn dedup_rules(
        collection: Vec<(Regex, Box<dyn Tag>)>,
        pretty_name: &str,
        warn_on_duplicates: bool,
    ) -> Vec<(Regex, Box<dyn Tag>)> {
        let mut filtered = vec![];
        let mut seen = BTreeMap::<String, usize>::new();

        let mut i = 0;
        for word in collection {
            if let Some(previous) = seen.get(word.0.as_str()) {
                if warn_on_duplicates {
                    log::warn!(
                        "{} already present at position {} in {}",
                        word.0,
                        previous,
                        pretty_name,
                    );
                }

                filtered[*previous] = word;
            } else {
                seen.insert(word.0.to_string(), i);
                filtered.push(word);
                i += 1;
            }
        }

        filtered
    }

    pub(crate) fn new(
        mut words: Vec<(Regex, Box<dyn Tag>)>,
        mut patterns: Vec<(Regex, Box<dyn Tag>)>,
        intensities_def: BTreeMap<u64, Intensity>,
    ) -> Self {
        words = Self::dedup_rules(words, "words", true);
        patterns = Self::dedup_rules(patterns, "patterns", true);

        let mut intensities = Vec::with_capacity(intensities_def.len());

        intensities.push((0, Self::merge_rules(&words, &patterns)));

        for (intensity, override_or_addition) in intensities_def {
            let rules = match override_or_addition {
                Intensity::Replace(overrides) => {
                    words = Self::dedup_rules(overrides.words, "words", true);
                    patterns = Self::dedup_rules(overrides.patterns, "patterns", true);

                    Self::merge_rules(&words, &patterns)
                }
                Intensity::Extend(additions) => {
                    // no duplicates are allowed inside new definitions
                    let new_words = Self::dedup_rules(additions.words, "words", true);
                    let new_patterns = Self::dedup_rules(additions.patterns, "patterns", true);

                    // NOTE: we do not just add everything to the end of `replacements`. words and
                    // patterns maintain relative order where words are always first
                    words.extend(new_words);
                    patterns.extend(new_patterns);

                    // we deduped old and new words separately, now they are merged. dedup again
                    // without warnings. new ones take priority over old while keeping position
                    words = Self::dedup_rules(words, "words", false);
                    patterns = Self::dedup_rules(patterns, "patterns", false);

                    Self::merge_rules(&words, &patterns)
                }
            };

            intensities.push((intensity, rules));
        }

        Self { intensities }
    }

    /// Returns all registered intensities in ascending order. Note that there may be gaps
    pub fn intensities(&self) -> Vec<u64> {
        self.intensities.iter().map(|(k, _)| *k).collect()
    }

    /// Walks rules for given intensity from top to bottom and applies them
    pub fn say_it<'a>(&self, text: &'a str, intensity: u64) -> Cow<'a, str> {
        // Go from the end and pick first intensity that is less or eaual to requested. This is
        // guaranteed to return something because base intensity 0 is always present at the bottom
        // and 0 <= x is true for any u64
        let rules = &self
            .intensities
            .iter()
            .rev()
            .find(|(current_intensity, _)| *current_intensity <= intensity)
            .expect("intensity 0 is always present")
            .1;

        let mut result = Cow::Borrowed(text);

        // apply rules from top to bottom
        for rule in rules {
            match rule.apply(&result) {
                Cow::Borrowed(_) => {}
                Cow::Owned(new) => result = Cow::from(new),
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::collections::BTreeMap;

    use crate::{
        intensity::{Intensity, IntensityBody},
        tag::{Literal, NoMimicCase},
        Accent,
    };

    #[test]
    fn e() {
        let e = Accent::new(
            vec![],
            vec![
                (
                    Regex::new(r"(?-i)[a-z]").unwrap(),
                    NoMimicCase::new_boxed(Literal::new_boxed("e")),
                ),
                (
                    Regex::new(r"(?-i)[A-Z]").unwrap(),
                    NoMimicCase::new_boxed(Literal::new_boxed("E")),
                ),
            ],
            BTreeMap::new(),
        );

        assert_eq!(e.say_it("Hello World!", 0), "Eeeee Eeeee!");
    }

    #[test]
    fn conflicting_pattern_in_same_intensity_is_replaced_and_warns() {
        let mut intensities = BTreeMap::new();
        intensities.insert(
            1,
            Intensity::Replace(IntensityBody {
                words: vec![],
                patterns: vec![
                    // second one overwrites first in intensity 1
                    (Regex::new("b").unwrap(), Literal::new_boxed("1")),
                    (Regex::new("b").unwrap(), Literal::new_boxed("2")),
                ],
            }),
        );

        let e = Accent::new(
            vec![],
            vec![
                // second one overwrites first in intensity 0
                (Regex::new("a").unwrap(), Literal::new_boxed("b")),
                (Regex::new("a").unwrap(), Literal::new_boxed("c")),
            ],
            intensities,
        );

        assert_eq!(e.say_it("abab", 0), "cbcb");
        assert_eq!(e.say_it("abab", 1), "a2a2");
    }
}
