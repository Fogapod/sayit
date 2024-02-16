use std::borrow::Cow;

use crate::pass::Pass;

/// Holds [`Pass`] objects and applies them in order
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Intensity {
    pub(crate) level: u64,
    passes: Vec<Pass>,
}

impl Intensity {
    pub fn new(level: u64, passes: Vec<Pass>) -> Self {
        Self { level, passes }
    }

    /// Produces new instance by extending inner passes
    pub fn extend(&self, level: u64, passes: Vec<Pass>) -> Result<Self, String> {
        let mut existing_passes = self.passes.clone();
        let mut appended_passes = Vec::new();

        for new_pass in passes {
            let mut replaced = false;

            for existing_pass in existing_passes.iter_mut() {
                if existing_pass.name == new_pass.name {
                    // FIXME: remove clone
                    *existing_pass = existing_pass.extend(new_pass.clone())?;
                    replaced = true;
                    break;
                }
            }

            if !replaced {
                appended_passes.push(new_pass);
            }
        }

        existing_passes.extend(appended_passes);

        Ok(Self::new(level, existing_passes))
    }

    /// Runs all inner passes against text
    pub fn apply<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.passes.iter().fold(Cow::Borrowed(text), |a, r| {
            Cow::Owned(r.apply(&a).into_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{intensity::Intensity, pass::Pass, tag::Literal};

    #[test]
    fn rules_replaced() {
        let old_pass = Pass::new(
            "",
            vec![
                ("old", Literal::new_boxed("old")),
                ("old2", Literal::new_boxed("old2")),
            ],
        )
        .unwrap();

        let new_pass = Pass::new("", vec![("old", Literal::new_boxed("new"))]).unwrap();

        let old = Intensity::new(0, vec![old_pass]);

        let extended = old.extend(1, vec![new_pass]).unwrap();
        let expected = Pass::new(
            "",
            vec![
                ("old", Literal::new_boxed("new")),
                ("old2", Literal::new_boxed("old2")),
            ],
        )
        .unwrap();

        assert_eq!(extended.level, 1);
        assert_eq!(extended.passes, vec![expected]);
    }

    #[test]
    fn rules_appended() {
        let old_pass = Pass::new("", vec![("existing", Literal::new_boxed("old"))]).unwrap();
        let new_pass = Pass::new("", vec![("added", Literal::new_boxed("new"))]).unwrap();

        let old = Intensity::new(0, vec![old_pass]);

        let extended = old.extend(1, vec![new_pass]).unwrap();
        let expected = Pass::new(
            "",
            vec![
                ("existing", Literal::new_boxed("old")),
                ("added", Literal::new_boxed("new")),
            ],
        )
        .unwrap();

        assert_eq!(extended.passes, vec![expected]);
    }
}
