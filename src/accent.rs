use crate::intensity::Intensity;

use std::{borrow::Cow, error::Error, fmt};

/// Replaces patterns in text according to rules
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Accent {
    // a set of rules for each intensity level, sorted from lowest to highest
    pub(crate) intensities: Vec<Intensity>,
}

#[derive(Debug, PartialEq)]
pub enum CreationError {
    IntensityZeroMissing,
    FirstIntensityNotZero,
    UnsortedOrDuplicatedIntensities(u64),
}

impl fmt::Display for CreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreationError::IntensityZeroMissing => {
                write!(f, "expected at least a base intensity 0")
            }
            CreationError::FirstIntensityNotZero => {
                write!(f, "first intensity must have level 0")
            }
            CreationError::UnsortedOrDuplicatedIntensities(level) => {
                write!(f, "duplicated or out of order intensity level {level}")
            }
        }
    }
}

impl Error for CreationError {}

impl Accent {
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

impl TryFrom<Vec<Intensity>> for Accent {
    type Error = CreationError;

    fn try_from(intensities: Vec<Intensity>) -> Result<Self, Self::Error> {
        if intensities.is_empty() {
            return Err(CreationError::IntensityZeroMissing);
        }

        if intensities[0].level != 0 {
            return Err(CreationError::FirstIntensityNotZero);
        }

        let mut seen = Vec::with_capacity(intensities.len());
        seen.push(0);

        for (i, intensity) in intensities[1..].iter().enumerate() {
            if intensity.level <= seen[i] {
                return Err(CreationError::UnsortedOrDuplicatedIntensities(
                    intensity.level,
                ));
            }
            seen.push(intensity.level);
        }

        // reverse now to not reverse each time to find highest matching intensity
        let intensities: Vec<_> = intensities.into_iter().rev().collect();

        Ok(Self { intensities })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        accent::CreationError, intensity::Intensity, pass::Pass, tag_impls::Literal, Accent,
    };

    #[test]
    fn e() {
        let base_intensity = Intensity::new(
            0,
            vec![(
                "".to_owned(),
                Pass::new(vec![
                    ("(?-i)[a-z]".to_string(), Literal::new_boxed("e")),
                    ("[A-Z]".to_string(), Literal::new_boxed("E")),
                ])
                .unwrap(),
            )],
        );
        let e = Accent::try_from(vec![base_intensity]).unwrap();

        assert_eq!(e.say_it("Hello World!", 0), "Eeeee Eeeee!");
    }

    #[test]
    fn construction_error_empty_intensities() {
        assert_eq!(
            Accent::try_from(Vec::new()).err().unwrap(),
            CreationError::IntensityZeroMissing
        );
    }

    #[test]
    fn construction_error_first_must_be_0() {
        let intensities = vec![Intensity::new(12, Vec::new())];

        assert_eq!(
            Accent::try_from(intensities).err().unwrap(),
            CreationError::FirstIntensityNotZero
        );
    }

    #[test]
    fn construction_error_out_of_order() {
        let intensities = vec![
            Intensity::new(0, Vec::new()),
            Intensity::new(3, Vec::new()),
            Intensity::new(1, Vec::new()),
        ];

        assert_eq!(
            Accent::try_from(intensities).err().unwrap(),
            CreationError::UnsortedOrDuplicatedIntensities(1)
        );
    }
}
