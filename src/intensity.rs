use std::borrow::Cow;

use crate::pass::{self, Pass};

/// Holds [`Pass`] objects and applies them in order
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Intensity {
    pub(crate) level: u64,
    names: Vec<String>,
    passes: Vec<Pass>,
}

impl Intensity {
    pub fn new(level: u64, passes: Vec<(String, Pass)>) -> Self {
        let (names, passes) = passes.into_iter().unzip();

        Self {
            level,
            names,
            passes,
        }
    }

    /// Produces new instance by extending inner passes
    #[allow(clippy::result_large_err)]
    pub fn extend(
        &self,
        level: u64,
        passes: Vec<(String, Pass)>,
    ) -> Result<Self, pass::CreationError> {
        let mut existing_passes: Vec<(String, Pass)> = self
            .names
            .clone()
            .into_iter()
            .zip(self.passes.clone())
            .collect();
        let mut appended_passes = Vec::new();

        for (new_name, new_pass) in passes {
            let mut replaced = false;

            for (existing_name, existing_pass) in &mut existing_passes {
                if *existing_name == new_name {
                    // FIXME: remove clone
                    *existing_pass = existing_pass.extend(new_pass.clone())?;
                    replaced = true;
                    break;
                }
            }

            if !replaced {
                appended_passes.push((new_name, new_pass));
            }
        }

        existing_passes.extend(appended_passes);

        Ok(Self::new(level, existing_passes))
    }

    /// Runs all inner passes against text
    pub fn apply<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.passes.iter().fold(Cow::Borrowed(text), |acc, pass| {
            Cow::Owned(pass.apply(&acc).into_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{intensity::Intensity, pass::Pass, tag_impls::Literal};

    #[test]
    fn rules_replaced() {
        let old_pass = Pass::new(vec![
            ("old".to_string(), Literal::new_boxed("old")),
            ("old2".to_string(), Literal::new_boxed("old2")),
        ])
        .unwrap();

        let new_pass = Pass::new(vec![("old".to_string(), Literal::new_boxed("new"))]).unwrap();

        let old = Intensity::new(0, vec![("".to_string(), old_pass)]);

        let extended = old.extend(1, vec![("".to_string(), new_pass)]).unwrap();
        let expected = Pass::new(vec![
            ("old".to_string(), Literal::new_boxed("new")),
            ("old2".to_string(), Literal::new_boxed("old2")),
        ])
        .unwrap();

        assert_eq!(extended.level, 1);
        assert_eq!(extended.passes, vec![expected]);
        assert_eq!(extended.names, vec!["".to_string()]);
    }

    #[test]
    fn rules_appended() {
        let old_pass =
            Pass::new(vec![("existing".to_string(), Literal::new_boxed("old"))]).unwrap();
        let new_pass = Pass::new(vec![("added".to_string(), Literal::new_boxed("new"))]).unwrap();

        let old = Intensity::new(0, vec![("".to_string(), old_pass)]);

        let extended = old.extend(1, vec![("".to_string(), new_pass)]).unwrap();
        let expected = Pass::new(vec![
            ("existing".to_string(), Literal::new_boxed("old")),
            ("added".to_string(), Literal::new_boxed("new")),
        ])
        .unwrap();

        assert_eq!(extended.passes, vec![expected]);
        assert_eq!(extended.names, vec!["".to_string()]);
    }

    #[test]
    fn passes_appended() {
        let old_pass = ("old".to_string(), Pass::new(Vec::new()).unwrap());
        let new_pass = ("new".to_string(), Pass::new(Vec::new()).unwrap());

        let old = Intensity::new(0, vec![old_pass]);

        let extended = old.extend(1, vec![new_pass]).unwrap();
        let expected = vec![
            Pass::new(Vec::new()).unwrap(),
            Pass::new(Vec::new()).unwrap(),
        ];

        assert_eq!(extended.passes, expected);
        assert_eq!(extended.names, vec!["old".to_string(), "new".to_string()]);
    }
}
