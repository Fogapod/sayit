#[cfg(feature = "deserialize")]
use crate::deserialize::AccentDef;

use crate::intensity::Intensity;

use std::borrow::Cow;

/// Replaces patterns in text according to rules
#[derive(Debug)]
#[cfg_attr(
    feature = "deserialize",
    derive(serde::Deserialize),
    serde(try_from = "AccentDef")
)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Accent {
    // a set of rules for each intensity level, sorted from lowest to highest
    pub(crate) intensities: Vec<Intensity>,
}

impl Accent {
    pub(crate) fn new(intensities: Vec<Intensity>) -> Result<Self, String> {
        if intensities.is_empty() {
            return Err("Expected at least a base intensity 0".to_owned());
        }

        Ok(Self { intensities })
    }

    /// Returns all registered intensities in ascending order. Note that there may be gaps
    pub fn intensities(&self) -> Vec<u64> {
        self.intensities.iter().map(|i| i.level).collect()
    }

    /// Walks rules for given intensity from top to bottom and applies them
    pub fn say_it<'a>(&self, text: &'a str, intensity: u64) -> Cow<'a, str> {
        // Go from the end and pick first intensity that is less or eaual to requested. This is
        // guaranteed to return something because base intensity 0 is always present at the bottom
        // and 0 <= x is true for any u64
        let intensity = &self
            .intensities
            .iter()
            .rev()
            .find(|i| i.level <= intensity)
            .expect("intensity 0 is always present");

        intensity.apply(text)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        intensity::Intensity,
        pass::Pass,
        tag::{Literal, NoMimicCase},
        Accent,
    };

    #[test]
    fn e() {
        let base_intensity = Intensity::new(
            0,
            vec![Pass::new(
                "",
                vec![
                    (
                        "(?-i)[a-z]".to_owned(),
                        NoMimicCase::new_boxed(Literal::new_boxed("e")),
                    ),
                    (
                        "[A-Z]".to_owned(),
                        NoMimicCase::new_boxed(Literal::new_boxed("E")),
                    ),
                ],
            )
            .unwrap()],
        );
        let e = Accent::new(vec![base_intensity]).unwrap();

        assert_eq!(e.say_it("Hello World!", 0), "Eeeee Eeeee!");
    }
}
