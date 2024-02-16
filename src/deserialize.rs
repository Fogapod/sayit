use crate::utils::runtime_format_single_value;
use std::fmt;

use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    accent::Accent,
    intensity::Intensity,
    pass::Pass,
    tag::{Any, AnyError, Tag, Weights, WeightsError},
};

// this is not strictly nescessary but implemented manually for consistent serde error message
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

// deserialize weights as map u64 -> Tag
impl<'de> Deserialize<'de> for Weights {
    fn deserialize<D>(deserializer: D) -> Result<Weights, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct WeightsVisitor;

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
                    WeightsError::ZeroItems => {
                        de::Error::invalid_length(0, &"at least one element")
                    }
                    WeightsError::NonPositiveTotalWeights => {
                        de::Error::custom("weights must add up to positive number")
                    }
                })
            }
        }

        deserializer.deserialize_map(WeightsVisitor)
    }
}

// deserializes like map but is a vec
struct RuleMap(Vec<(String, Box<dyn Tag>)>);

impl<'de> Deserialize<'de> for RuleMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RuleMapVisitor;

        impl<'de> Visitor<'de> for RuleMapVisitor {
            type Value = RuleMap;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("rules: `regex: Tag`")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut data = Vec::with_capacity(access.size_hint().unwrap_or(0));
                let mut seen_regexes = Vec::with_capacity(data.capacity());

                while let Some((regex, tag)) = access.next_entry::<String, Box<dyn Tag>>()? {
                    let regex_str = regex.as_str().to_owned();
                    if seen_regexes.contains(&regex_str) {
                        return Err(de::Error::custom(format!("duplicated regex: {regex_str}")));
                    }
                    seen_regexes.push(regex_str);

                    data.push((regex, tag));
                }

                Ok(RuleMap(data))
            }
        }

        deserializer.deserialize_map(RuleMapVisitor)
    }
}

fn default_pass_format() -> String {
    "{}".to_owned()
}

#[derive(Deserialize)]
pub(crate) struct PassDef {
    name: String,
    #[serde(default = "default_pass_format")]
    format: String,
    rules: RuleMap,
}

impl TryFrom<PassDef> for Pass {
    type Error = String;

    fn try_from(value: PassDef) -> Result<Self, Self::Error> {
        let mut rules = value.rules.0;

        for rule in rules.iter_mut() {
            rule.0 = runtime_format_single_value(&value.format, &rule.0)?;
        }

        Self::new(&value.name, rules)
    }
}

#[derive(Deserialize)]
enum IntensityDef {
    Replace(Vec<Pass>),
    Extend(Vec<Pass>),
}

#[derive(Default)]
struct IntensitiesDef(Vec<(u64, IntensityDef)>);

impl<'de> Deserialize<'de> for IntensitiesDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IntensitiesVisitor;

        impl<'de> Visitor<'de> for IntensitiesVisitor {
            type Value = IntensitiesDef;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("intensities: `1: Intensity`")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut intensities: Vec<(u64, IntensityDef)> =
                    Vec::with_capacity(access.size_hint().unwrap_or(0));

                while let Some((level, intensity)) = access.next_entry()? {
                    if level == 0 {
                        return Err(de::Error::custom("intensity cannot be 0"));
                    }

                    for (seen_level, _) in &intensities {
                        if seen_level == &level {
                            return Err(de::Error::custom(format!(
                                "duplicate intensity level: {seen_level}"
                            )));
                        }

                        let passes = match &intensity {
                            IntensityDef::Replace(passes) | IntensityDef::Extend(passes) => passes,
                        };

                        let mut seen_passes = Vec::with_capacity(passes.len());
                        for pass in passes {
                            if seen_passes.contains(&pass.name) {
                                return Err(de::Error::custom(format!(
                                    "duplicate pass name: {}",
                                    pass.name
                                )));
                            }
                            seen_passes.push(pass.name.clone());
                        }
                    }
                    if intensities.iter().any(|(l, _)| l == &level) {
                        return Err(de::Error::duplicate_field("intensity"));
                    }

                    if let Some(last) = intensities.last() {
                        if last.0 > level {
                            return Err(de::Error::custom(format!(
                                "intensities out of order: {} > {}",
                                last.0, level
                            )));
                        }
                    }

                    intensities.push((level, intensity));
                }

                Ok(IntensitiesDef(intensities))
            }
        }

        deserializer.deserialize_map(IntensitiesVisitor)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AccentDef {
    accent: Vec<Pass>,
    #[serde(default)]
    intensities: IntensitiesDef,
}

impl TryFrom<AccentDef> for Accent {
    type Error = String;

    fn try_from(accent_def: AccentDef) -> Result<Self, Self::Error> {
        let mut intensities: Vec<Intensity> =
            Vec::with_capacity(accent_def.intensities.0.len() + 1);

        intensities.push(Intensity::new(0, accent_def.accent));

        for (i, (level, intensity)) in accent_def.intensities.0.into_iter().enumerate() {
            let intensity = match intensity {
                IntensityDef::Replace(passes) => Intensity::new(level, passes),
                IntensityDef::Extend(passes) => intensities[i].extend(level, passes)?,
            };

            intensities.push(intensity);
        }

        Self::new(intensities)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        intensity::Intensity,
        pass::{Match, Pass},
        tag::{Any, Literal, Original, Tag, Weights},
        Accent,
    };

    #[test]
    fn ron_minimal() {
        let _ = ron::from_str::<Accent>("(accent: [])").unwrap();
    }

    #[test]
    fn ron_empty() {
        let _ = ron::from_str::<Accent>(r#"(accent: [(name: "", rules: {})], intensities: {})"#)
            .unwrap();
    }

    #[test]
    fn ron_extend_extends() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    accent: [
        (
            name: "words",
            format: r"\b{}\b",
            rules: {"a": {"Original": ()}},
        ),
        (
            name: "patterns",
            rules: {"1": {"Original": ()}},
        ),
    ],
    intensities: {
        1: Extend([
            (
                name: "words",
                format: r"\b{}\b",
                rules: {"b": {"Original": ()}},
            ),
            (
                name: "patterns",
                rules: {"2": {"Original": ()}},
            ),
        ]),
    },
)
"#,
        )
        .unwrap();

        let manual = vec![
            Intensity::new(
                0,
                vec![
                    Pass::new("words", vec![(r"\ba\b", Original::new_boxed())]).unwrap(),
                    Pass::new("patterns", vec![("1", Original::new_boxed())]).unwrap(),
                ],
            ),
            Intensity::new(
                1,
                vec![
                    Pass::new(
                        "words",
                        vec![
                            (r"\ba\b", Original::new_boxed()),
                            (r"\bb\b", Original::new_boxed()),
                        ],
                    )
                    .unwrap(),
                    Pass::new(
                        "patterns",
                        vec![("1", Original::new_boxed()), ("2", Original::new_boxed())],
                    )
                    .unwrap(),
                ],
            ),
        ];

        let accent = Accent::new(manual).unwrap();

        assert_eq!(parsed, accent);
        assert_eq!(parsed.intensities(), accent.intensities());
    }

    #[test]
    fn ron_replace_replaces() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    accent: [
        (
            name: "words",
            format: r"\b{}\b",
            rules: {"a": {"Original": ()}},
        ),
        (
            name: "patterns",
            rules: {"1": {"Original": ()}},
        ),
    ],
    intensities: {
        1: Replace([
            (
                name: "words",
                format: r"\b{}\b",
                rules: {"b": {"Original": ()}},
            ),
            (
                name: "patterns",
                rules: {"2": {"Original": ()}},
            ),
        ]),
    },
)
"#,
        )
        .unwrap();

        let intensities = vec![
            Intensity::new(
                0,
                vec![
                    Pass::new("words", vec![(r"\ba\b", Original::new_boxed())]).unwrap(),
                    Pass::new("patterns", vec![("1", Original::new_boxed())]).unwrap(),
                ],
            ),
            Intensity::new(
                1,
                vec![
                    Pass::new("words", vec![(r"\bb\b", Original::new_boxed())]).unwrap(),
                    Pass::new("patterns", vec![("2", Original::new_boxed())]).unwrap(),
                ],
            ),
        ];

        let manual = Accent::new(intensities).unwrap();

        assert_eq!(parsed, manual);
    }

    #[test]
    fn ron_invalid_tag_any() {
        let empty = ron::from_str::<Any>("[]").err().unwrap();
        assert_eq!(
            empty.code.to_string(),
            "Expected at least one element but found zero elements instead"
        );
    }

    #[test]
    fn ron_invalid_tag_weighted() {
        let empty = ron::from_str::<Weights>("{}").err().unwrap();

        let zero_sum = ron::from_str::<Weights>(
            r#"
{
    0: {"Original": ()},
    0: {"Original": ()},
    0: {"Original": ()},
},
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
    fn ron_malformed() {
        assert!(ron::from_str::<Accent>(r#"("borken..."#).is_err());
    }

    #[test]
    fn ron_all_features() {
        let ron_string = r#"
(
    accent: [
        (
            name: "words",
            format: r"\b{}\b",
            rules: {
                "test": {"Literal": "Testing in progress; Please ignore ..."},
                "badword": {"Literal": ""},
                "dupe": {"Literal": "0"},
            },
        ),
        (
            name: "patterns",
            rules: {
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
        ),
    ],
    intensities: {
        1: Replace([
            (
                name: "words",
                format: r"\b{}\b",
                rules: {
                    "replaced": {"Literal": "words"},
                    "dupe": {"Literal": "1"},
                    "Windows": {"Literal": "Linux"},
                },
            ),
            (
                name: "patterns",
                rules: {
                    "a+": {"Literal": "multiple A's"},
                    "^": {"Literal": "start"},
                },
            ),
        ]),
        2: Extend([
            (
                name: "words",
                format: r"\b{}\b",
                rules: {
                    "dupe": {"Literal": "2"},
                    "added": {"Literal": "words"},
                },
            ),
            (
                name: "patterns",
                rules: {
                    "b+": {"Literal": "multiple B's"},
                    "$": {"Literal": "end"},
                },
            ),
        ]),
    },
)
"#;

        let parsed = ron::from_str::<Accent>(ron_string).unwrap();
        let intensities = vec![
            Intensity::new(
                0,
                vec![
                    Pass::new(
                        "words",
                        vec![
                            (
                                r"\btest\b",
                                Literal::new_boxed("Testing in progress; Please ignore ..."),
                            ),
                            (r"\bbadword\b", Literal::new_boxed("")),
                            (r"\bdupe\b", Literal::new_boxed("0")),
                        ],
                    )
                    .unwrap(),
                    Pass::new(
                        "patterns",
                        vec![
                            (r"[a-z]", Literal::new_boxed("e")),
                            (
                                r"[A-Z]",
                                Weights::new_boxed(vec![
                                    (5, Literal::new_boxed("E")),
                                    (1, Literal::new_boxed("Ē")),
                                    (1, Literal::new_boxed("Ê")),
                                    (1, Literal::new_boxed("Ë")),
                                    (1, Literal::new_boxed("È")),
                                    (1, Literal::new_boxed("É")),
                                ])
                                .unwrap(),
                            ),
                            (
                                r"[0-9]",
                                Any::new_boxed(vec![Weights::new_boxed(vec![(
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
                            ),
                        ],
                    )
                    .unwrap(),
                ],
            ),
            Intensity::new(
                1,
                vec![
                    Pass::new(
                        "words",
                        vec![
                            (r"\breplaced\b", Literal::new_boxed("words")),
                            (r"\bdupe\b", Literal::new_boxed("1")),
                            (r"\bWindows\b", Literal::new_boxed("Linux")),
                        ],
                    )
                    .unwrap(),
                    Pass::new(
                        "patterns",
                        vec![
                            (r"a+", Literal::new_boxed("multiple A's")),
                            (r"^", Literal::new_boxed("start")),
                        ],
                    )
                    .unwrap(),
                ],
            ),
            Intensity::new(
                2,
                vec![
                    Pass::new(
                        "words",
                        vec![
                            (r"\breplaced\b", Literal::new_boxed("words")),
                            (r"\bdupe\b", Literal::new_boxed("2")),
                            (r"\bWindows\b", Literal::new_boxed("Linux")),
                            (r"\badded\b", Literal::new_boxed("words")),
                        ],
                    )
                    .unwrap(),
                    Pass::new(
                        "patterns",
                        vec![
                            (r"a+", Literal::new_boxed("multiple A's")),
                            (r"^", Literal::new_boxed("start")),
                            (r"b+", Literal::new_boxed("multiple B's")),
                            (r"$", Literal::new_boxed("end")),
                        ],
                    )
                    .unwrap(),
                ],
            ),
        ];
        let manual = Accent::new(intensities).unwrap();
        assert_eq!(manual, parsed);

        // TODO: either patch rand::thread_rng somehow or change interface to pass rng directly?
        // let test_string = "Hello World! test 12 23";
        // for intensity in manual.intensities() {
        //     assert_eq!(parsed.say_it(test_string, intensity), manual.say_it(test_string, intensity));
        //  }
    }

    #[test]
    fn pass_duplicated_regexes_now_allowed() {
        let err = ron::from_str::<Pass>(
            r#"
(
    name: "somename",
    rules: {
        "dupew": {"Literal": "0"},
        "dupew": {"Literal": "1"},
        "dupew": {"Literal": "2"},
    }
)
"#,
        )
        .err()
        .unwrap();

        assert_eq!(err.code.to_string(), "duplicated regex: dupew");
    }

    #[test]
    fn intensity_0_not_allowed() {
        assert_eq!(
            ron::from_str::<Accent>(r#"(accent: [], intensities: { 0: Extend([]) })"#)
                .err()
                .unwrap()
                .code
                .to_string(),
            "intensity cannot be 0"
        );
    }

    #[test]
    fn intensity_selection() {
        let accent = ron::from_str::<Accent>(
            r#"
(
    accent: [
        (
            name: "words",
            format: r"\b{}\b",
            rules: {"intensity": {"Literal": "0"}},
        ),
    ],
    intensities: {
        1: Replace([
            (
                name: "words",
                format: r"\b{}\b",
                rules: {"intensity": {"Literal": "1"}},
            ),
        ]),
        5: Replace([
            (
                name: "words",
                format: r"\b{}\b",
                rules: {"intensity": {"Literal": "5"}},
            ),
        ]),
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
            fn generate<'a>(&self, m: &Match<'a>) -> std::borrow::Cow<'a, str> {
                let input = m.get_match();

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
    accent: [
        (
            name: "patterns",
            rules: {
                r"\d+": {"Increment": (101)},
            },
        ),
    ]
)
"#,
        )
        .unwrap();

        assert_eq!(accent.say_it("565 0", 0), "666 101");
    }
}
