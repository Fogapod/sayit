use crate::{
    accent::Accent,
    intensity::{Intensity, IntensityBody},
    tag::{Any, AnyError, Tag, Weights, WeightsError},
    utils::LiteralString,
};

use std::{collections::BTreeMap, fmt, marker::PhantomData, sync::OnceLock};

use regex::Regex;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};

impl<'de> Deserialize<'de> for LiteralString {
    fn deserialize<D>(deserializer: D) -> Result<LiteralString, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;

        Ok(Self::from(s))
    }
}

impl<'de> Deserialize<'de> for Any {
    fn deserialize<D>(deserializer: D) -> Result<Any, D::Error>
    where
        D: Deserializer<'de>,
    {
        let items: Vec<Box<dyn Tag>> = Deserialize::deserialize(deserializer)?;

        Self::new(items).map_err(|err| match err {
            AnyError::ZeroItems => de::Error::invalid_length(0, &"at least one element"),
        })
    }
}

struct WeightsVisitor {}

impl WeightsVisitor {
    fn new() -> Self {
        Self {}
    }
}

impl<'de> Visitor<'de> for WeightsVisitor {
    type Value = Weights;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("weights: `1: Tag`")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut data = Vec::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((key, value)) = access.next_entry()? {
            data.push((key, value));
        }

        Weights::new(data).map_err(|err| match err {
            WeightsError::ZeroItems => de::Error::invalid_length(0, &"at least one element"),
            WeightsError::NonPositiveTotalWeights => {
                de::Error::custom("weights must add up to positive number")
            }
        })
    }
}

impl<'de> Deserialize<'de> for Weights {
    fn deserialize<D>(deserializer: D) -> Result<Weights, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(WeightsVisitor::new())
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

static REGEX_FLAGS_REGEX: OnceLock<Regex> = OnceLock::new();

impl TryFrom<String> for WordRegex {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let regex_flags = if s.chars().all(|c| c.is_ascii_lowercase()) {
            "mi"
        } else {
            "m"
        };

        // a hack to extract regex flags from words. they do not work since they would be placed
        // after \b without this
        let flags_regex =
            REGEX_FLAGS_REGEX.get_or_init(|| Regex::new(r"^(:?\(\?-?[a-zA-Z]+\))+").unwrap());

        let (maybe_regex_flags_from_string, s) = match flags_regex.captures(&s) {
            Some(caps) => (caps.get(0).unwrap().as_str(), flags_regex.replace(&s, "")),
            None => ("", s.into()),
        };

        Ok(Self(
            Regex::new(&format!(
                r"(?{regex_flags}){maybe_regex_flags_from_string}\b{s}\b"
            ))
            .map_err(|err| err.to_string())?,
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

/// Deserializes as map but is actually a vec of (K, V) to preserve order
struct RuleMap<T> {
    ordered: Vec<(T, Box<dyn Tag>)>,
}

impl<T> Default for RuleMap<T> {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl<T> RuleMap<T> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            ordered: Vec::with_capacity(capacity),
        }
    }
}

struct RuleMapVisitor<T> {
    marker: PhantomData<fn() -> RuleMap<T>>,
}

impl<T> RuleMapVisitor<T> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de, T> Visitor<'de> for RuleMapVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = RuleMap<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(r#"rule map: `"regex": Tag`"#)
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = RuleMap::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((key, value)) = access.next_entry()? {
            map.ordered.push((key, value));
        }

        Ok(map)
    }
}

impl<'de, T> Deserialize<'de> for RuleMap<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(RuleMapVisitor::new())
    }
}

// this exists separately and not flattened because ron does not support serde(flatten)
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IntensityBodyDef {
    #[serde(default)]
    words: RuleMap<WordRegex>,
    #[serde(default)]
    patterns: RuleMap<PatternRegex>,
}

impl From<IntensityBodyDef> for IntensityBody {
    fn from(intensity_def: IntensityBodyDef) -> Self {
        Self {
            words: intensity_def
                .words
                .ordered
                .into_iter()
                .map(|(regex, tag)| (regex.0, tag))
                .collect(),
            patterns: intensity_def
                .patterns
                .ordered
                .into_iter()
                .map(|(regex, tag)| (regex.0, tag))
                .collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AccentDef {
    #[serde(default)]
    words: RuleMap<WordRegex>,
    #[serde(default)]
    patterns: RuleMap<PatternRegex>,
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
                .ordered
                .into_iter()
                .map(|(regex, tag)| (regex.0, tag))
                .collect(),
            accent_def
                .patterns
                .ordered
                .into_iter()
                .map(|(regex, tag)| (regex.0, tag))
                .collect(),
            accent_def.intensities,
        ))
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::{
        deserialize::WordRegex,
        rule::Rule,
        tag::{Any, Literal, Original, Tag, Weights},
        Accent,
    };

    #[test]
    fn ron_minimal() {
        let _ = ron::from_str::<Accent>("()").unwrap();
    }

    #[test]
    fn ron_empty() {
        let _ = ron::from_str::<Accent>(r#"(words: {}, patterns: {}, intensities: {})"#).unwrap();
    }

    #[test]
    fn ron_extend_extends() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    words: {"a": {"Original": ()}},
    patterns: {"1": {"Original": ()}},
    intensities: {
        1: Extend(
            (
                words: {"b": {"Original": ()}},
                patterns: {"2": {"Original": ()}},
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
                            tag: Original::new_boxed(),
                        },
                        Rule {
                            source: Regex::new("(?m)1").unwrap(),
                            tag: Original::new_boxed(),
                        },
                    ],
                ),
                (
                    1,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\ba\b").unwrap(),
                            tag: Original::new_boxed(),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bb\b").unwrap(),
                            tag: Original::new_boxed(),
                        },
                        Rule {
                            source: Regex::new("(?m)1").unwrap(),
                            tag: Original::new_boxed(),
                        },
                        Rule {
                            source: Regex::new("(?m)2").unwrap(),
                            tag: Original::new_boxed(),
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
    words: {"a": {"Original": ()}},
    patterns: {"1": {"Original": ()}},
    intensities: {
        1: Replace((
            words: {"b": {"Original": ()}},
            patterns: {"2": {"Original": ()}},
        )),
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
                            tag: Original::new_boxed(),
                        },
                        Rule {
                            source: Regex::new("(?m)1").unwrap(),
                            tag: Original::new_boxed(),
                        },
                    ],
                ),
                (
                    1,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\bb\b").unwrap(),
                            tag: Original::new_boxed(),
                        },
                        Rule {
                            source: Regex::new("(?m)2").unwrap(),
                            tag: Original::new_boxed(),
                        },
                    ],
                ),
            ],
        };

        assert_eq!(parsed, manual);
    }

    #[test]
    fn ron_invalid_tag_any() {
        let empty = ron::from_str::<Accent>(
            r#"
(
    patterns: {
        "a": {"Any": []}
    }
)
"#,
        )
        .err()
        .unwrap();
        assert_eq!(
            empty.code.to_string(),
            "invalid length 0, expected at least one element"
        );
    }

    #[test]
    fn ron_invalid_tag_weighted() {
        let empty = ron::from_str::<Accent>(
            r#"
(
    patterns: {
        "a": {"Weights": {}}
    }
)
"#,
        )
        .err()
        .unwrap();

        let zero_sum = ron::from_str::<Accent>(
            r#"
(
    patterns: {
        "a": {"Weights": {
            0: {"Original": ()},
            0: {"Original": ()},
            0: {"Original": ()},
        }},
    }
)
"#,
        )
        .err()
        .unwrap();

        assert_eq!(
            empty.code.to_string(),
            "Expected at least one element but found zero elements instead"
        );

        assert_eq!(
            zero_sum.code.to_string(),
            "weights must add up to positive number"
        );
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
    words: {
        "test": {"Literal": "Testing in progress; Please ignore ..."},
        "badword": {"Literal": ""},
        "dupe": {"Literal": "0"},
    },
    patterns: {
        // lowercase letters are replaced with e
        "[a-z]": {"Literal": "e"},
        // uppercase letters are replaced with 50% uppercase "E" and 10% for each of the cursed "E"
        "[A-Z]": {"Weights": {
            5: {"Literal": "E"},
            1: {"Literal": "Ē"},
            1: {"Literal": "Ê"},
            1: {"Literal": "Ë"},
            1: {"Literal": "È"},
            1: {"Literal": "É"},
        }},
        // numbers are replaced with 6 or 9 or are left untouched
        // excessive nesting that does nothing
        "[0-9]": {"Any": [
            {"Weights": {
                1: {"Any": [
                      {"Literal": "6"},
                      {"Literal": "9"},
                      {"Original": ()},
                ]},
            }},
        ]},
    },
    intensities: {
        1: Replace((
            words: {
                "replaced": {"Literal": "words"},
                "dupe": {"Literal": "1"},
                "Windows": {"Literal": "Linux"},
            },
            patterns: {
                "a+": {"Literal": "multiple A's"},
                "^": {"Literal": "start"},
            },
        )),
        2: Extend((
            words: {
                "dupe": {"Literal": "2"},
                "added": {"Literal": "words"},
            },
            patterns: {
                "b+": {"Literal": "multiple B's"},
                "$": {"Literal": "end"},
            },
        )),
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
                            tag: Literal::new_boxed("Testing in progress; Please ignore ..."),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bbadword\b").unwrap(),
                            tag: Literal::new_boxed(""),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bdupe\b").unwrap(),
                            tag: Literal::new_boxed("0"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)[a-z]").unwrap(),
                            tag: Literal::new_boxed("e"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)[A-Z]").unwrap(),
                            tag: Weights::new_boxed(vec![
                                (5, Literal::new_boxed("E")),
                                (1, Literal::new_boxed("Ē")),
                                (1, Literal::new_boxed("Ê")),
                                (1, Literal::new_boxed("Ë")),
                                (1, Literal::new_boxed("È")),
                                (1, Literal::new_boxed("É")),
                            ])
                            .unwrap(),
                        },
                        Rule {
                            source: Regex::new(r"(?m)[0-9]").unwrap(),
                            tag: Any::new_boxed(vec![Weights::new_boxed(vec![(
                                1,
                                Any::new_boxed(vec![
                                    Literal::new_boxed("6"),
                                    Literal::new_boxed("9"),
                                    Original::new_boxed(),
                                ])
                                .unwrap(),
                            )])
                            .unwrap()])
                            .unwrap(),
                        },
                    ],
                ),
                (
                    1,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\breplaced\b").unwrap(),
                            tag: Literal::new_boxed("words"),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bdupe\b").unwrap(),
                            tag: Literal::new_boxed("1"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)\bWindows\b").unwrap(),
                            tag: Literal::new_boxed("Linux"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)a+").unwrap(),
                            tag: Literal::new_boxed("multiple A's"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)^").unwrap(),
                            tag: Literal::new_boxed("start"),
                        },
                    ],
                ),
                (
                    2,
                    vec![
                        Rule {
                            source: Regex::new(r"(?mi)\breplaced\b").unwrap(),
                            tag: Literal::new_boxed("words"),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\bdupe\b").unwrap(),
                            tag: Literal::new_boxed("2"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)\bWindows\b").unwrap(),
                            tag: Literal::new_boxed("Linux"),
                        },
                        Rule {
                            source: Regex::new(r"(?mi)\badded\b").unwrap(),
                            tag: Literal::new_boxed("words"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)a+").unwrap(),
                            tag: Literal::new_boxed("multiple A's"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)^").unwrap(),
                            tag: Literal::new_boxed("start"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)b+").unwrap(),
                            tag: Literal::new_boxed("multiple B's"),
                        },
                        Rule {
                            source: Regex::new(r"(?m)$").unwrap(),
                            tag: Literal::new_boxed("end"),
                        },
                    ],
                ),
            ],
        };
        assert_eq!(manual, parsed);

        // TODO: either patch rand::thread_rng somehow or change interface to pass rng directly?
        // let test_string = "Hello World! test 12 23";
        // for intensity in manual.intensities() {
        //     assert_eq!(parsed.say_it(test_string, intensity), manual.say_it(test_string, intensity));
        //  }
    }

    #[test]
    fn duplicates_eliminated() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    words: {
        "dupew": {"Literal": "0"},
        "dupew": {"Literal": "1"},
        "dupew": {"Literal": "2"},
    },
    patterns: {
        "dupep": {"Literal": "0"},
        "dupep": {"Literal": "1"},
        "dupep": {"Literal": "2"},
    },
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
                        tag: Literal::new_boxed("2"),
                    },
                    Rule {
                        source: Regex::new(r"(?mi)dupep").unwrap(),
                        tag: Literal::new_boxed("2"),
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
    words: {"intensity": {"Literal": "0"}},
    intensities: {
        1: Replace((
            words: {"intensity": {"Literal": "1"}},
        )),
        5: Replace((
            words: {"intensity": {"Literal": "5"}},
        )),
    },
)
"#,
        )
        .unwrap();

        assert_eq!(accent.say_it("intensity", 0), "0");
        assert_eq!(accent.say_it("intensity", 1), "1");
        assert_eq!(accent.say_it("intensity", 4), "1");
        assert_eq!(accent.say_it("intensity", 5), "5");
        assert_eq!(accent.say_it("intensity", 9000 + 1), "5");
    }

    #[test]
    fn custom_tag_works() {
        /// Increments matched number by given amount. Does nothing for overflow or bad match
        #[derive(Clone, Debug, serde::Deserialize)]
        pub struct Increment(u32);

        #[typetag::deserialize]
        impl Tag for Increment {
            fn generate<'a>(
                &self,
                caps: &regex::Captures,
                input: &'a str,
            ) -> std::borrow::Cow<'a, str> {
                let input = self.current_match(caps, input);

                let input_number: i64 = match input.parse() {
                    Ok(parsed) => parsed,
                    Err(_) => return input.into(),
                };

                match input_number.checked_add(self.0 as i64) {
                    Some(added) => added.to_string().into(),
                    None => input.into(),
                }
            }
        }

        let accent = ron::from_str::<Accent>(
            r#"
(
    patterns: {
        r"\d+": {"Increment": (101)},
    }
)
"#,
        )
        .unwrap();

        assert_eq!(accent.say_it("565 0", 0), "666 101");
    }

    #[test]
    fn regex_flags_are_moved_in_word_regex() {
        let s = "(?i)(?U)(?-Ri)(dw)test".to_owned();

        assert_eq!(
            WordRegex::try_from(s).unwrap().0.as_str(),
            r"(?m)(?i)(?U)(?-Ri)\b(dw)test\b"
        );
    }
}
