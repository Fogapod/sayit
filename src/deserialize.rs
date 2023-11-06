use crate::accent::Accent;
use crate::replacement::{AnyReplacement, ReplacementCallback, SimpleString, WeightedReplacement};
use crate::severity::Severity;

use serde::{de, Deserialize, Deserializer};
use std::collections::BTreeMap;

impl<'de> Deserialize<'de> for SimpleString {
    fn deserialize<D>(deserializer: D) -> Result<SimpleString, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(SimpleString::new(Deserialize::deserialize(deserializer)?))
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

        Ok(AnyReplacement(items))
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

        Ok(WeightedReplacement(weights))
    }
}

impl TryFrom<AccentDef> for Accent {
    type Error = String;

    fn try_from(accent_def: AccentDef) -> Result<Self, Self::Error> {
        Self::new(
            accent_def.normalize_case,
            accent_def.words,
            accent_def.patterns,
            accent_def.severities,
        )
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
    words: Vec<(String, ReplacementCallback)>,
    #[serde(default)]
    patterns: Vec<(String, ReplacementCallback)>,
    #[serde(default)]
    severities: BTreeMap<u64, Severity>,
}
