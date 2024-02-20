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
    pub fn new(intensities: Vec<Intensity>) -> Result<Self, String> {
        if intensities.is_empty() {
            return Err("Expected at least a base intensity 0".to_owned());
        }

        if intensities[0].level != 0 {
            return Err("First intensity must have level 0".to_owned());
        }

        let mut seen = Vec::with_capacity(intensities.len());
        seen.push(0);

        for (i, intensity) in intensities[1..].iter().enumerate() {
            if intensity.level <= seen[i] {
                return Err(format!("Duplicated or out of order intensity level {i}"));
            }
            seen.push(intensity.level);
        }

        // reverse now to not reverse each time to find highest matching intensity
        let intensities: Vec<_> = intensities.into_iter().rev().collect();

        Ok(Self { intensities })
    }

    /// Returns all registered intensities in ascending order. Note that there may be gaps
    pub fn intensities(&self) -> Vec<u64> {
        self.intensities.iter().rev().map(|i| i.level).collect()
    }

    /// Walks rules for given intensity from top to bottom and applies them
    pub fn say_it<'a>(&self, text: &'a str, intensity: u64) -> Cow<'a, str> {
        // Go from the end and pick first intensity that is less or eaual to requested. This is
        // guaranteed to return something because base intensity 0 is always present at the bottom
        // and 0 <= x is true for any u64
        let intensity = &self
            .intensities
            .iter()
            .find(|i| i.level <= intensity)
            .expect("intensity 0 is always present");

        intensity.apply(text)
    }
}

#[cfg(test)]
mod tests {
    use crate::{intensity::Intensity, pass::Pass, tag_impls::Literal, Accent};

    #[test]
    fn e() {
        let base_intensity = Intensity::new(
            0,
            vec![Pass::new(
                "",
                vec![
                    ("(?-i)[a-z]".to_owned(), Literal::new_boxed("e")),
                    ("[A-Z]".to_owned(), Literal::new_boxed("E")),
                ],
            )
            .unwrap()],
        );
        let e = Accent::new(vec![base_intensity]).unwrap();

        assert_eq!(e.say_it("Hello World!", 0), "Eeeee Eeeee!");
    }

    #[test]
    fn construction_error_empty_intensities() {
        assert!(Accent::new(Vec::new()).is_err());
    }

    #[test]
    fn construction_error_first_must_be_0() {
        let intensities = vec![Intensity::new(12, Vec::new())];

        assert!(Accent::new(intensities).is_err());
    }

    #[test]
    fn construction_error_out_of_order() {
        let intensities = vec![
            Intensity::new(0, Vec::new()),
            Intensity::new(3, Vec::new()),
            Intensity::new(1, Vec::new()),
        ];

        assert!(Accent::new(intensities).is_err());
    }
}
