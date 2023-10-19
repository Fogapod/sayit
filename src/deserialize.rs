use crate::accent::Accent;
use crate::replacement::{ReplacementCallback, SimpleString};
use crate::severity::Severity;
use crate::severity::SeverityBody;

use serde::Deserialize;
use std::collections::BTreeMap;

impl TryFrom<ReplacementCallbackDef> for ReplacementCallback {
    type Error = String;

    // TODO: these should be in Deserialize implementation when/if it is done
    fn try_from(accent_def: ReplacementCallbackDef) -> Result<Self, Self::Error> {
        Ok(match accent_def {
            ReplacementCallbackDef::Noop => Self::Noop,
            ReplacementCallbackDef::Simple(body) => Self::Simple(SimpleString::new(&body)),
            ReplacementCallbackDef::Any(items) => {
                if items.is_empty() {
                    return Err("Empty Any".to_owned());
                }

                let mut converted = Vec::with_capacity(items.len());
                for item in items {
                    converted.push(item.try_into()?);
                }
                ReplacementCallback::Any(converted)
            }
            ReplacementCallbackDef::Weights(items) => {
                if items.is_empty() {
                    return Err("Empty Weights".to_owned());
                }

                if items.iter().map(|(i, _)| i).sum::<u64>() == 0 {
                    return Err("Weights add up to 0".to_owned());
                }

                let mut converted = Vec::with_capacity(items.len());
                for (weight, item) in items {
                    converted.push((weight, item.try_into()?));
                }
                Self::Weights(converted)
            }
        })
    }
}

#[derive(Debug, Deserialize)]
pub(crate) enum ReplacementCallbackDef {
    Noop,
    Simple(String),
    Any(Vec<ReplacementCallbackDef>),
    Weights(Vec<(u64, ReplacementCallbackDef)>),
}

impl TryFrom<SeverityBodyDef> for SeverityBody {
    type Error = String;

    // TODO: these should be in Deserialize implementation when/if it is done
    fn try_from(accent_def: SeverityBodyDef) -> Result<Self, Self::Error> {
        let mut words = Vec::with_capacity(accent_def.words.len());
        for (i, (pattern, callback_def)) in accent_def.words.into_iter().enumerate() {
            let callback: ReplacementCallback = match callback_def.try_into() {
                Err(err) => Err(format!("error in word {i}: {pattern}: {err}"))?,
                Ok(callback) => callback,
            };
            words.push((pattern, callback));
        }

        let mut patterns = Vec::with_capacity(accent_def.patterns.len());
        for (i, (pattern, callback_def)) in accent_def.patterns.into_iter().enumerate() {
            let callback: ReplacementCallback = match callback_def.try_into() {
                Err(err) => Err(format!("error in pattern {i}: {pattern}: {err}"))?,
                Ok(callback) => callback,
            };
            patterns.push((pattern, callback));
        }

        Ok(Self { words, patterns })
    }
}

impl TryFrom<SeverityDef> for Severity {
    type Error = String;

    // TODO: these should be in Deserialize implementation when/if it is done
    fn try_from(severity_def: SeverityDef) -> Result<Self, Self::Error> {
        Ok(match severity_def {
            SeverityDef::Replace(body) => Self::Replace(body.try_into()?),
            SeverityDef::Extend(body) => Self::Extend(body.try_into()?),
        })
    }
}
#[derive(Debug, Deserialize)]
pub(crate) struct SeverityBodyDef {
    #[serde(default)]
    pub(crate) words: Vec<(String, ReplacementCallbackDef)>,
    #[serde(default)]
    pub(crate) patterns: Vec<(String, ReplacementCallbackDef)>,
}

#[derive(Debug, Deserialize)]
pub(crate) enum SeverityDef {
    Replace(SeverityBodyDef),
    Extend(SeverityBodyDef),
}

fn default_bool_true() -> bool {
    true
}

impl TryFrom<AccentDef> for Accent {
    type Error = String;

    // NOTE: this should all go away with custom Deserialize hopefully?
    fn try_from(accent_def: AccentDef) -> Result<Self, Self::Error> {
        let mut words = Vec::with_capacity(accent_def.words.len());
        for (i, (pattern, callback_def)) in accent_def.words.into_iter().enumerate() {
            let callback: ReplacementCallback = match callback_def.try_into() {
                Err(err) => Err(format!("error in word {i}: {pattern}: {err}"))?,
                Ok(callback) => callback,
            };
            words.push((pattern, callback));
        }

        let mut patterns = Vec::with_capacity(accent_def.patterns.len());
        for (i, (pattern, callback_def)) in accent_def.patterns.into_iter().enumerate() {
            let callback: ReplacementCallback = match callback_def.try_into() {
                Err(err) => Err(format!("error in pattern {i}: {pattern}: {err}"))?,
                Ok(callback) => callback,
            };
            patterns.push((pattern, callback));
        }

        let mut severities = BTreeMap::new();
        for (severity_level, severity_def) in accent_def.severities.into_iter() {
            let severity: Severity = match severity_def.try_into() {
                Err(err) => Err(format!("error in severity {severity_level}: {err}"))?,
                Ok(callback) => callback,
            };

            severities.insert(severity_level, severity);
        }

        Self::new(accent_def.normalize_case, words, patterns, severities)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AccentDef {
    #[serde(default = "default_bool_true")]
    pub(crate) normalize_case: bool,
    #[serde(default)]
    pub(crate) words: Vec<(String, ReplacementCallbackDef)>,
    #[serde(default)]
    pub(crate) patterns: Vec<(String, ReplacementCallbackDef)>,
    #[serde(default)]
    pub(crate) severities: BTreeMap<u64, SeverityDef>,
}
