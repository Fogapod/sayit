#[cfg(feature = "deserialize")]
use crate::deserialize::AccentDef;
use crate::intensity::Intensity;
use crate::replacement::Replacement;
use crate::rule::Rule;

use std::borrow::Cow;
use std::collections::BTreeMap;

use regex::Regex;

/// Replaces patterns in text according to rules
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
#[cfg_attr(feature = "deserialize", serde(try_from = "AccentDef"))]
pub struct Accent {
    // a set of rules for each intensity level, sorted from lowest to highest
    pub(crate) intensities: Vec<(u64, Vec<Rule>)>,
}

impl Accent {
    fn merge_rules(first: &[(Regex, Replacement)], second: &[(Regex, Replacement)]) -> Vec<Rule> {
        first
            .iter()
            .chain(second)
            .map(|(regex, replacement)| Rule {
                source: regex.clone(),
                replacement: replacement.clone(),
            })
            .collect()
    }

    // keeps collection order, rewrites left duplicates with right ones
    // TODO: investigate the usefulness of defining same pattern multiple times. Since rules are
    //       sequentional, are there situations when we might want to apply something on top of
    //       another change? `"*": Lowercase(Original); ...; Lowercase(Original)` this might be a
    //       hacky way to fix something in complex accents
    fn dedup_rules(
        collection: Vec<(Regex, Replacement)>,
        pretty_name: &str,
        warn_on_duplicates: bool,
    ) -> Vec<(Regex, Replacement)> {
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
        mut words: Vec<(Regex, Replacement)>,
        mut patterns: Vec<(Regex, Replacement)>,
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
    pub fn apply<'a>(&self, text: &'a str, intensity: u64) -> Cow<'a, str> {
        // TODO: binary search? probably now worth
        //
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

    use crate::replacement::Replacement;
    use crate::Accent;

    #[test]
    fn e() {
        let e = Accent::new(
            vec![],
            vec![
                (
                    Regex::new(r"(?-i)[a-z]").unwrap(),
                    Replacement::new_no_mimic_case(Replacement::new_simple("e")),
                ),
                (
                    Regex::new(r"(?-i)[A-Z]").unwrap(),
                    Replacement::new_no_mimic_case(Replacement::new_simple("E")),
                ),
            ],
            BTreeMap::new(),
        );

        assert_eq!(e.apply("Hello World!", 0), "Eeeee Eeeee!");
    }
}
