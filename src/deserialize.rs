use crate::accent::Accent;
use crate::replacement::{AnyReplacement, ReplacementCallback, SimpleString, WeightedReplacement};
use crate::severity::{Severity, SeverityBody};

use regex::Regex;
use serde::{de, Deserialize, Deserializer};
use std::collections::BTreeMap;

impl<'de> Deserialize<'de> for SimpleString {
    fn deserialize<D>(deserializer: D) -> Result<SimpleString, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self::new(Deserialize::deserialize(deserializer)?))
    }
}

impl<'de> Deserialize<'de> for AnyReplacement {
    fn deserialize<D>(deserializer: D) -> Result<AnyReplacement, D::Error>
    where
        D: Deserializer<'de>,
    {
        let items: Vec<ReplacementCallback> = Deserialize::deserialize(deserializer)?;
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
        let weights: Vec<(u64, ReplacementCallback)> = Deserialize::deserialize(deserializer)?;
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
pub(crate) struct SeverityBodyDef {
    #[serde(default)]
    words: Vec<(WordRegex, ReplacementCallback)>,
    #[serde(default)]
    patterns: Vec<(PatternRegex, ReplacementCallback)>,
}

impl From<SeverityBodyDef> for SeverityBody {
    fn from(severity_def: SeverityBodyDef) -> Self {
        Self {
            words: severity_def
                .words
                .into_iter()
                .map(|(regex, replacement)| (regex.0, replacement))
                .collect(),
            patterns: severity_def
                .patterns
                .into_iter()
                .map(|(regex, replacement)| (regex.0, replacement))
                .collect(),
        }
    }
}

fn default_bool_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub(crate) struct AccentDef {
    #[serde(default = "default_bool_true")]
    normalize_case: bool,
    #[serde(default)]
    words: Vec<(WordRegex, ReplacementCallback)>,
    #[serde(default)]
    patterns: Vec<(PatternRegex, ReplacementCallback)>,
    #[serde(default)]
    severities: BTreeMap<u64, Severity>,
}

impl TryFrom<AccentDef> for Accent {
    type Error = &'static str;

    fn try_from(accent_def: AccentDef) -> Result<Self, Self::Error> {
        if accent_def.severities.contains_key(&0) {
            return Err("severity cannot be 0 since 0 is base one");
        }

        Ok(Self::new(
            accent_def.normalize_case,
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
            accent_def.severities,
        ))
    }
}
