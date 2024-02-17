use crate::{tag::Tag, utils::runtime_format_single_value};
use std::{
    fmt::{self, Display},
    marker::PhantomData,
};

use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    accent::Accent,
    intensity::Intensity,
    pass::Pass,
    tag_impls::{Any, AnyError, Weights, WeightsError},
};

// deserializes from map while preserving order of elements
pub(crate) struct SortedMap<K, V, const UNIQUE: bool>(Vec<(K, V)>)
where
    K: PartialEq;

impl<'de, K, V, const UNIQUE: bool> Deserialize<'de> for SortedMap<K, V, { UNIQUE }>
where
    K: Deserialize<'de> + PartialEq + Display,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SortedMapVisitor<K, V, const U: bool>
        where
            K: PartialEq + Display,
        {
            marker: PhantomData<fn() -> SortedMap<K, V, { U }>>,
        }

        impl<K: PartialEq + Display, V, const U: bool> SortedMapVisitor<K, V, U> {
            fn new() -> Self {
                SortedMapVisitor {
                    marker: PhantomData,
                }
            }
        }

        impl<'de, K, V, const UNIQUE: bool> Visitor<'de> for SortedMapVisitor<K, V, UNIQUE>
        where
            K: Deserialize<'de> + PartialEq + Display,
            V: Deserialize<'de>,
        {
            type Value = SortedMap<K, V, { UNIQUE }>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("map")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut ordered = Vec::with_capacity(access.size_hint().unwrap_or(0));

                while let Some((key, value)) = access.next_entry()? {
                    if UNIQUE && ordered.iter().any(|(k, _)| k == &key) {
                        return Err(de::Error::custom(format!("duplicated key: {key}")));
                    }

                    ordered.push((key, value));
                }

                Ok(SortedMap(ordered))
            }
        }

        deserializer.deserialize_map(SortedMapVisitor::new())
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

impl TryFrom<SortedMap<u64, Box<dyn Tag>, false>> for Weights {
    type Error = WeightsError;

    fn try_from(value: SortedMap<u64, Box<dyn Tag>, false>) -> Result<Self, Self::Error> {
        Self::new(value.0)
    }
}

fn default_pass_format() -> String {
    "{}".to_owned()
}

#[derive(Deserialize)]
pub(crate) struct PassDef {
    #[serde(default = "default_pass_format")]
    format: String,
    rules: SortedMap<String, Box<dyn Tag>, true>,
}

struct Passes(Vec<Pass>);

impl<'de> Deserialize<'de> for Passes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let items = SortedMap::<String, PassDef, true>::deserialize(deserializer)?.0;
        let mut passes: Vec<Pass> = Vec::with_capacity(items.len());

        for (name, pass_def) in items {
            let mut rules = Vec::with_capacity(pass_def.rules.0.len());

            for (regex, tag) in pass_def.rules.0 {
                rules.push((
                    runtime_format_single_value(&pass_def.format, &regex)
                        .map_err(de::Error::custom)?,
                    tag,
                ));
            }

            passes.push(Pass::new(&name, rules).map_err(de::Error::custom)?);
        }

        Ok(Self(passes))
    }
}

#[derive(Deserialize)]
enum IntensityDef {
    Replace(Passes),
    Extend(Passes),
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
                let mut intensities = Vec::with_capacity(access.size_hint().unwrap_or(0));

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
    accent: Passes,
    #[serde(default)]
    intensities: IntensitiesDef,
}

impl TryFrom<AccentDef> for Accent {
    type Error = String;

    fn try_from(accent_def: AccentDef) -> Result<Self, Self::Error> {
        let mut intensities: Vec<Intensity> =
            Vec::with_capacity(accent_def.intensities.0.len() + 1);

        intensities.push(Intensity::new(0, accent_def.accent.0));

        for (i, (level, intensity)) in accent_def.intensities.0.into_iter().enumerate() {
            let intensity = match intensity {
                IntensityDef::Replace(passes) => Intensity::new(level, passes.0),
                IntensityDef::Extend(passes) => intensities[i].extend(level, passes.0)?,
            };

            intensities.push(intensity);
        }

        Self::new(intensities)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        deserialize::Passes,
        intensity::Intensity,
        pass::Pass,
        tag::Tag,
        tag_impls::{Any, Literal, Original, Weights},
        Accent, Match,
    };

    #[test]
    fn ron_minimal() {
        let _ = ron::from_str::<Accent>("(accent: {})").unwrap();
    }

    #[test]
    fn ron_empty() {
        let _ =
            ron::from_str::<Accent>(r#"(accent: { "": ( rules: {} ) }, intensities: {})"#).unwrap();
    }

    #[test]
    fn ron_extend_extends() {
        let parsed = ron::from_str::<Accent>(
            r#"
(
    accent: {
        "words": (
            format: r"\b{}\b",
            rules: {"a": {"Original": ()}},
        ),
        "patterns": (
            rules: {"1": {"Original": ()}},
        ),
    },
    intensities: {
        1: Extend({
            "words": (
                format: r"\b{}\b",
                rules: {"b": {"Original": ()}},
            ),
            "patterns": (
                rules: {"2": {"Original": ()}},
            ),
        }),
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
    accent: {
        "words": (
            format: r"\b{}\b",
            rules: {"a": {"Original": ()}},
        ),
        "patterns": (
            rules: {"1": {"Original": ()}},
        ),
    },
    intensities: {
        1: Replace({
            "words": (
                format: r"\b{}\b",
                rules: {"b": {"Original": ()}},
            ),
            "patterns": (
                rules: {"2": {"Original": ()}},
            ),
        }),
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
            zero_sum.code.to_string(),
            "Weights must add up to a positive number"
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
    accent: {
        "words": (
            format: r"\b{}\b",
            rules: {
                "test": {"Literal": "Testing in progress; Please ignore ..."},
                "badword": {"Literal": ""},
                "dupe": {"Literal": "0"},
            },
        ),
        "patterns": (
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
    },
    intensities: {
        1: Replace({
            "words": (
                format: r"\b{}\b",
                rules: {
                    "replaced": {"Literal": "words"},
                    "dupe": {"Literal": "1"},
                    "Windows": {"Literal": "Linux"},
                },
            ),
            "patterns": (
                rules: {
                    "a+": {"Literal": "multiple A's"},
                    "^": {"Literal": "start"},
                },
            ),
        }),
        2: Extend({
            "words": (
                format: r"\b{}\b",
                rules: {
                    "dupe": {"Literal": "2"},
                    "added": {"Literal": "words"},
                },
            ),
            "patterns": (
                rules: {
                    "b+": {"Literal": "multiple B's"},
                    "$": {"Literal": "end"},
                },
            ),
        }),
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
        let err = ron::from_str::<Passes>(
            r#"
{
    "somename":
        (
            rules: {
                "dupew": {"Literal": "0"},
                "dupew": {"Literal": "1"},
                "dupew": {"Literal": "2"},
            }
        )
}
"#,
        )
        .err()
        .unwrap();

        assert_eq!(err.code.to_string(), "duplicated key: dupew");
    }

    #[test]
    fn intensity_0_not_allowed() {
        assert_eq!(
            ron::from_str::<Accent>(r#"(accent: {}, intensities: { 0: Extend({}) })"#)
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
    accent: {
        "words": (
            format: r"\b{}\b",
            rules: {"intensity": {"Literal": "0"}},
        ),
    },
    intensities: {
        1: Replace({
            "words": (
                name: "words",
                format: r"\b{}\b",
                rules: {"intensity": {"Literal": "1"}},
            ),
        }),
        5: Replace({
            "words": (
                format: r"\b{}\b",
                rules: {"intensity": {"Literal": "5"}},
            ),
        }),
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
    accent: {
        "patterns": (
            name: "patterns",
            rules: {
                r"\d+": {"Increment": (101)},
            },
        ),
    }
)
"#,
        )
        .unwrap();

        assert_eq!(accent.say_it("565 0", 0), "666 101");
    }
}
