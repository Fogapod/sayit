#[cfg(feature = "deserialize")]
use crate::deserialize::AccentDef;
use crate::replacement::{Replacement, Rule};
use crate::severity::Severity;

use std::collections::BTreeMap;

use regex::Regex;

/// Replaces patterns in text according to rules
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "deserialize", derive(serde::Deserialize))]
#[cfg_attr(feature = "deserialize", serde(try_from = "AccentDef"))]
pub struct Accent {
    normalize_case: bool,
    // a set of rules for each severity level, sorted from lowest to highest
    severities: Vec<(u64, Vec<Rule>)>,
}

impl Accent {
    fn merge_rules(
        first: Vec<(Regex, Replacement)>,
        second: Vec<(Regex, Replacement)>,
    ) -> Vec<Rule> {
        first
            .into_iter()
            .chain(second)
            .map(|(regex, replacement)| Rule {
                source: regex,
                replacement,
            })
            .collect()
    }

    // keeps collection order, rewrites left duplicates with right ones
    fn dedup_rules(
        collection: Vec<(Regex, Replacement)>,
        pretty_name: &str,
        drop_expected: bool,
    ) -> Vec<(Regex, Replacement)> {
        let mut filtered = vec![];
        let mut seen = BTreeMap::<String, usize>::new();

        let mut i = 0;
        for word in collection {
            if let Some(previous) = seen.get(word.0.as_str()) {
                filtered[*previous] = word.clone();
                if !drop_expected {
                    log::warn!(
                        "{} already present at position {} in {}",
                        word.0,
                        previous,
                        pretty_name,
                    );
                }
            } else {
                seen.insert(word.0.to_string(), i);
                filtered.push(word);
                i += 1;
            }
        }

        filtered
    }

    pub(crate) fn new(
        normalize_case: bool,
        mut words: Vec<(Regex, Replacement)>,
        mut patterns: Vec<(Regex, Replacement)>,
        severities_def: BTreeMap<u64, Severity>,
    ) -> Self {
        words = Self::dedup_rules(words, "words", false);
        patterns = Self::dedup_rules(patterns, "patterns", false);

        let mut severities = Vec::with_capacity(severities_def.len());

        severities.push((0, Self::merge_rules(words.clone(), patterns.clone())));

        for (severity, override_or_addition) in severities_def {
            let rules = match override_or_addition {
                Severity::Replace(overrides) => {
                    words = Self::dedup_rules(overrides.words, "words", false);
                    patterns = Self::dedup_rules(overrides.patterns, "patterns", false);

                    Self::merge_rules(words.clone(), patterns.clone())
                }
                Severity::Extend(additions) => {
                    // no duplicates are allowed inside new definitions
                    let new_words = Self::dedup_rules(additions.words, "words", false);
                    let new_patterns = Self::dedup_rules(additions.patterns, "patterns", false);

                    // NOTE: we do not just add everything to the end of `replacements`. words and
                    // patterns maintain relative order where words are always first
                    words.extend(new_words);
                    patterns.extend(new_patterns);

                    // we deduped old and new words separately, now they are merged. dedup again
                    // without warnings. new ones take priority over old while keeping position
                    words = Self::dedup_rules(words, "words", true);
                    patterns = Self::dedup_rules(patterns, "patterns", true);

                    Self::merge_rules(words.clone(), patterns.clone())
                }
            };

            severities.push((severity, rules));
        }

        Self {
            normalize_case,
            severities,
        }
    }

    /// Returns all registered severities in ascending order. Note that there may be gaps
    pub fn severities(&self) -> Vec<u64> {
        self.severities.iter().map(|(k, _)| *k).collect()
    }

    /// Walks rules for given severity from top to bottom and applies them
    pub fn apply(&self, text: &str, severity: u64) -> String {
        // TODO: binary search? probably now worth
        //
        // Go from the end and pick first severity that is less or eaual to requested. This is
        // guaranteed to return something because base severity 0 is always present at the bottom
        // and 0 <= x is true for any u64
        let replacements = &self
            .severities
            .iter()
            .rev()
            .find(|(sev, _)| *sev <= severity)
            .expect("severity 0 is always present")
            .1;

        let mut result = text.to_owned();

        // apply rules from top to bottom
        for replacement in replacements {
            result = replacement.apply(&result, self.normalize_case);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::{collections::BTreeMap, fs, vec};

    use crate::replacement::{Replacement, Rule};
    use crate::Accent;

    #[test]
    fn e() {
        let e = Accent::new(
            false,
            vec![],
            vec![
                (
                    Regex::new(r"(?-i)[a-z]").unwrap(),
                    Replacement::new_simple("e"),
                ),
                (
                    Regex::new(r"(?-i)[A-Z]").unwrap(),
                    Replacement::new_simple("E"),
                ),
            ],
            BTreeMap::new(),
        );

        assert_eq!(e.apply("Hello World!", 0), "Eeeee Eeeee!");
    }

    #[test]
    fn ron_minimal() {
        let _ = ron::from_str::<Accent>("()").unwrap();
    }

    #[test]
    fn ron_empty() {
        let _ = ron::from_str::<Accent>(r#"(words: [], patterns: [], severities: {})"#).unwrap();
    }

    #[test]
    fn ron_extend_extends() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    words: [("a", Original)],
    patterns: [("1", Original)],
    severities: {
        1: Extend(
            (
                words: [("b", Original)],
                patterns: [("2", Original)],
            )

        ),
    },
)
"#,
        )
        .unwrap();

        let manual = Accent {
            normalize_case: true,
            severities: vec![
                (
                    0,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\ba\b").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                        Rule {
                            source: Regex::new("(?m)1").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                    ],
                ),
                (
                    1,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\ba\b").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bb\b").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                        Rule {
                            source: Regex::new("(?m)1").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                        Rule {
                            source: Regex::new("(?m)2").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                    ],
                ),
            ],
        };

        assert_eq!(parsed, manual);
        assert_eq!(parsed.severities(), manual.severities());
    }

    #[test]
    fn ron_replace_replaces() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    words: [("a", Original)],
    patterns: [("1", Original)],
    severities: {
        1: Replace(
            (
                words: [("b", Original)],
                patterns: [("2", Original)],
            )

        ),
    },
)
"#,
        )
        .unwrap();

        let manual = Accent {
            normalize_case: true,
            severities: vec![
                (
                    0,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\ba\b").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                        Rule {
                            source: Regex::new("(?m)1").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                    ],
                ),
                (
                    1,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\bb\b").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                        Rule {
                            source: Regex::new("(?m)2").unwrap(),
                            replacement: Replacement::new_original(),
                        },
                    ],
                ),
            ],
        };

        assert_eq!(parsed, manual);
    }

    #[test]
    fn ron_invalid_replacement_any() {
        assert!(ron::from_str::<Accent>(
            r#"
(
    patterns:
        [
            ("a", Any([]))
        ]
)
"#
        )
        .err()
        .unwrap()
        .to_string()
        .contains("at least one element"));
    }

    #[test]
    fn ron_invalid_replacement_weighted() {
        assert!(ron::from_str::<Accent>(
            r#"
(
    patterns:
        [
            ("a", Weights([]))
        ]
)
"#
        )
        .err()
        .unwrap()
        .to_string()
        .contains("at least one element"));

        assert!(ron::from_str::<Accent>(
            r#"
(
    patterns:
        [
            ("a", Weights(
                [
                    (0, Original),
                    (0, Original),
                ]
            ))
        ]
)
"#
        )
        .err()
        .unwrap()
        .to_string()
        .contains("weights must add up to positive number"));
    }

    #[test]
    fn ron_severity_starts_from_0() {
        assert!(
            ron::from_str::<Accent>(r#"(severities: { 0: Extend(()) })"#)
                .err()
                .unwrap()
                .to_string()
                .contains("severity cannot be 0")
        );
    }

    #[test]
    fn ron_malformed() {
        assert!(ron::from_str::<Accent>(r#"("borken..."#).is_err());
    }

    #[test]
    fn ron_all_features() {
        let ron_string = r#"
(
    normalize_case: true,
    words: [
        ("test", Simple("Testing in progress; Please ignore ...")),
        ("badword", Simple("")),
        ("dupe", Simple("0")),
    ],
    patterns: [
        // lowercase letters are replaced with e
        ("[a-z]", Simple("e")),
        // uppercase letters are replaced with 50% uppercase "E" and 10% for each of the cursed "E"
        ("[A-Z]", Weights(
            [
                (5, Simple("E")),
                (1, Simple("Ē")),
                (1, Simple("Ê")),
                (1, Simple("Ë")),
                (1, Simple("È")),
                (1, Simple("É")),
            ],
        )),
        // numbers are replaced with 6 or 9 or are left untouched
        // excessive nesting that does nothing
        ("[0-9]", Any(
            [
                Weights(
                    [
                        (1, Any(
                            [
                              Simple("6"),
                              Simple("9"),
                              Original,
                            ],
                        )),
                    ],
                ),
            ],
        )),
    ],
    severities: {
        1: Replace(
            (
                words: [
                    ("replaced", Simple("words")),
                    ("dupe", Simple("1")),
                    ("Windows", Simple("Linux")),
                ],
                patterns: [
                    ("a+", Simple("multiple A's")),
                    ("^", Simple("start")),
                ],
            )
        ),
        2: Extend(
            (
                words: [
                    ("dupe", Simple("2")),
                    ("added", Simple("words")),
                ],
                patterns: [
                    ("b+", Simple("multiple B's")),
                    ("$", Simple("end")),
                ],
            )
        ),
    },
)
"#;

        let parsed = ron::from_str::<Accent>(ron_string).unwrap();
        let manual = Accent {
            normalize_case: true,
            severities: vec![
                (
                    0,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\btest\b").unwrap(),
                            replacement: Replacement::new_simple(
                                "Testing in progress; Please ignore ...",
                            ),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bbadword\b").unwrap(),
                            replacement: Replacement::new_simple(""),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bdupe\b").unwrap(),
                            replacement: Replacement::new_simple("0"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)[a-z]").unwrap(),
                            replacement: Replacement::new_simple("e"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)[A-Z]").unwrap(),
                            replacement: Replacement::new_weights(vec![
                                (5, Replacement::new_simple("E")),
                                (1, Replacement::new_simple("Ē")),
                                (1, Replacement::new_simple("Ê")),
                                (1, Replacement::new_simple("Ë")),
                                (1, Replacement::new_simple("È")),
                                (1, Replacement::new_simple("É")),
                            ]),
                        },
                        Rule {
                            source: Regex::new(r"(?m)[0-9]").unwrap(),
                            replacement: Replacement::new_any(vec![
                                Replacement::new_weights(vec![(
                                    1,
                                    Replacement::new_any(vec![
                                        Replacement::new_simple("6"),
                                        Replacement::new_simple("9"),
                                        Replacement::new_original(),
                                    ]),
                                )]),
                            ]),
                        },
                    ],
                ),
                (
                    1,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\breplaced\b").unwrap(),
                            replacement: Replacement::new_simple("words"),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bdupe\b").unwrap(),
                            replacement: Replacement::new_simple("1"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)\bWindows\b").unwrap(),
                            replacement: Replacement::new_simple("Linux"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)a+").unwrap(),
                            replacement: Replacement::new_simple("multiple A's"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)^").unwrap(),
                            replacement: Replacement::new_simple("start"),
                        },
                    ],
                ),
                (
                    2,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\breplaced\b").unwrap(),
                            replacement: Replacement::new_simple("words"),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bdupe\b").unwrap(),
                            replacement: Replacement::new_simple("2"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)\bWindows\b").unwrap(),
                            replacement: Replacement::new_simple("Linux"),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\badded\b").unwrap(),
                            replacement: Replacement::new_simple("words"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)a+").unwrap(),
                            replacement: Replacement::new_simple("multiple A's"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)^").unwrap(),
                            replacement: Replacement::new_simple("start"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)b+").unwrap(),
                            replacement: Replacement::new_simple("multiple B's"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)$").unwrap(),
                            replacement: Replacement::new_simple("end"),
                        },
                    ],
                ),
            ],
        };
        assert_eq!(manual, parsed);

        // TODO: either patch rand::thread_rng somehow or change interface to pass rng directly?
        // let test_string = "Hello World! test 12 23";
        // for severity in manual.severities() {
        //     assert_eq!(parsed.apply(test_string, severity), manual.apply(test_string, severity));
        //  }
    }

    #[test]
    fn duplicates_eliminated() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    words: [
        ("dupew", Simple("0")),
        ("dupew", Simple("1")),
        ("dupew", Simple("2")),
    ],
    patterns: [
        ("dupep", Simple("0")),
        ("dupep", Simple("1")),
        ("dupep", Simple("2")),
    ],
)
"#,
        )
        .unwrap();

        let manual = Accent {
            normalize_case: true,
            severities: vec![(
                0,
                vec![
                    Rule {
                        source: Regex::new(r"(?mi)\bdupew\b").unwrap(),
                        replacement: Replacement::new_simple("2"),
                    },
                    Rule {
                        source: Regex::new(r"(?mi)dupep").unwrap(),
                        replacement: Replacement::new_simple("2"),
                    },
                ],
            )],
        };

        assert_eq!(parsed, manual);
    }

    #[test]
    fn severity_selection() {
        let accent = ron::from_str::<Accent>(
            r#"
(
    words: [("severity", Simple("0"))],
    severities: {
        1: Replace(
            (
                words: [("severity", Simple("1"))],
            )

        ),
        5: Replace(
            (
                words: [("severity", Simple("5"))],
            )

        ),
    },
)
"#,
        )
        .unwrap();

        assert_eq!(accent.apply("severity", 0), "0");
        assert_eq!(accent.apply("severity", 1), "1");
        assert_eq!(accent.apply("severity", 4), "1");
        assert_eq!(accent.apply("severity", 5), "5");
        assert_eq!(accent.apply("severity", 9000 + 1), "5");
    }

    #[test]
    fn example_accents() {
        let sample_text = fs::read_to_string("tests/sample_text.txt").expect("reading sample text");

        for file in fs::read_dir("examples").expect("read symlinked accents folder") {
            let filename = file.expect("getting file info").path();
            println!("parsing {}", filename.display());

            let accent =
                ron::from_str::<Accent>(&fs::read_to_string(filename).expect("reading file"))
                    .unwrap();

            for severity in accent.severities() {
                let _ = accent.apply(&sample_text, severity);
            }
        }
    }
}
