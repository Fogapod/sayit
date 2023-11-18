use crate::accent::Accent;
use crate::intensity::{Intensity, IntensityBody};
use crate::replacement::{AnyReplacement, Replacement, WeightedReplacement};
use crate::utils::SimpleString;

use regex::Regex;
use serde::{de, Deserialize, Deserializer};
use std::collections::BTreeMap;

impl<'de> Deserialize<'de> for SimpleString {
    fn deserialize<D>(deserializer: D) -> Result<SimpleString, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;

        Ok(Self::from(s))
    }
}

impl<'de> Deserialize<'de> for AnyReplacement {
    fn deserialize<D>(deserializer: D) -> Result<AnyReplacement, D::Error>
    where
        D: Deserializer<'de>,
    {
        let items: Vec<Replacement> = Deserialize::deserialize(deserializer)?;
        if items.is_empty() {
            return Err(de::Error::invalid_length(0, &"at least one element"));
        }

        Ok(Self(items))
    }
}

impl<'de> Deserialize<'de> for WeightedReplacement {
    fn deserialize<D>(deserializer: D) -> Result<WeightedReplacement, D::Error>
    where
        D: Deserializer<'de>,
    {
        let weights: Vec<(u64, Replacement)> = Deserialize::deserialize(deserializer)?;
        if weights.is_empty() {
            return Err(de::Error::invalid_length(0, &"at least one element"));
        }
        if weights.iter().map(|(i, _)| i).sum::<u64>() == 0 {
            return Err(de::Error::custom("weights must add up to positive number"));
        }

        Ok(Self(weights))
    }
}

impl<'de> Deserialize<'de> for WordRegex {
    fn deserialize<D>(deserializer: D) -> Result<WordRegex, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;

        Self::try_from(s).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for PatternRegex {
    fn deserialize<D>(deserializer: D) -> Result<PatternRegex, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;

        Self::try_from(s).map_err(de::Error::custom)
    }
}

#[derive(Debug)]
struct WordRegex(Regex);

#[derive(Debug)]
struct PatternRegex(Regex);

impl TryFrom<String> for WordRegex {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let regex_flags = if s.chars().all(|c| c.is_ascii_lowercase()) {
            "mi"
        } else {
            "m"
        };

        Ok(Self(
            Regex::new(&format!(r"(?{regex_flags})\b{s}\b")).map_err(|err| err.to_string())?,
        ))
    }
}

impl TryFrom<String> for PatternRegex {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let regex_flags = if s.chars().all(|c| c.is_ascii_lowercase()) {
            "mi"
        } else {
            "m"
        };

        Ok(Self(
            Regex::new(&format!(r"(?{regex_flags}){s}")).map_err(|err| err.to_string())?,
        ))
    }
}

// this exists separately and not flattened because ron does not support serde(flatten)
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IntensityBodyDef {
    #[serde(default)]
    words: Vec<(WordRegex, Replacement)>,
    #[serde(default)]
    patterns: Vec<(PatternRegex, Replacement)>,
}

impl From<IntensityBodyDef> for IntensityBody {
    fn from(intensity_def: IntensityBodyDef) -> Self {
        Self {
            words: intensity_def
                .words
                .into_iter()
                .map(|(regex, replacement)| (regex.0, replacement))
                .collect(),
            patterns: intensity_def
                .patterns
                .into_iter()
                .map(|(regex, replacement)| (regex.0, replacement))
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AccentDef {
    #[serde(default)]
    words: Vec<(WordRegex, Replacement)>,
    #[serde(default)]
    patterns: Vec<(PatternRegex, Replacement)>,
    #[serde(default)]
    intensities: BTreeMap<u64, Intensity>,
}

impl TryFrom<AccentDef> for Accent {
    type Error = &'static str;

    fn try_from(accent_def: AccentDef) -> Result<Self, Self::Error> {
        if accent_def.intensities.contains_key(&0) {
            return Err("intensity cannot be 0 since 0 is base one");
        }

        Ok(Self::new(
            accent_def
                .words
                .into_iter()
                .map(|(regex, replacement)| (regex.0, replacement))
                .collect(),
            accent_def
                .patterns
                .into_iter()
                .map(|(regex, replacement)| (regex.0, replacement))
                .collect(),
            accent_def.intensities,
        ))
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::fs;

    use crate::replacement::Replacement;
    use crate::rule::Rule;
    use crate::Accent;

    #[test]
    fn ron_minimal() {
        let _ = ron::from_str::<Accent>("()").unwrap();
    }

    #[test]
    fn ron_empty() {
        let _ = ron::from_str::<Accent>(r#"(words: [], patterns: [], intensities: {})"#).unwrap();
    }

    #[test]
    fn ron_extend_extends() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    words: [("a", Original)],
    patterns: [("1", Original)],
    intensities: {
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
            intensities: vec![
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
        assert_eq!(parsed.intensities(), manual.intensities());
    }

    #[test]
    fn ron_replace_replaces() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    words: [("a", Original)],
    patterns: [("1", Original)],
    intensities: {
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
            intensities: vec![
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
    fn ron_intensity_starts_from_0() {
        assert!(
            ron::from_str::<Accent>(r#"(intensities: { 0: Extend(()) })"#)
                .err()
                .unwrap()
                .to_string()
                .contains("intensity cannot be 0")
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
    intensities: {
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
            intensities: vec![
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
                            replacement: Replacement::new_any(vec![Replacement::new_weights(
                                vec![(
                                    1,
                                    Replacement::new_any(vec![
                                        Replacement::new_simple("6"),
                                        Replacement::new_simple("9"),
                                        Replacement::new_original(),
                                    ]),
                                )],
                            )]),
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
        // for intensity in manual.intensities() {
        //     assert_eq!(parsed.apply(test_string, intensity), manual.apply(test_string, intensity));
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
            intensities: vec![(
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
    fn intensity_selection() {
        let accent = ron::from_str::<Accent>(
            r#"
(
    words: [("intensity", Simple("0"))],
    intensities: {
        1: Replace(
            (
                words: [("intensity", Simple("1"))],
            )

        ),
        5: Replace(
            (
                words: [("intensity", Simple("5"))],
            )

        ),
    },
)
"#,
        )
        .unwrap();

        assert_eq!(accent.apply("intensity", 0), "0");
        assert_eq!(accent.apply("intensity", 1), "1");
        assert_eq!(accent.apply("intensity", 4), "1");
        assert_eq!(accent.apply("intensity", 5), "5");
        assert_eq!(accent.apply("intensity", 9000 + 1), "5");
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

            for intensity in accent.intensities() {
                let _ = accent.apply(&sample_text, intensity);
            }
        }
    }
}
