use crate::Match;

use std::{borrow::Cow, fmt::Debug};

use dyn_clone::{clone_trait_object, DynClone};

/// Receives match and provides replacement
#[cfg_attr(feature = "deserialize", typetag::deserialize)]
pub trait Tag: DynClone + Debug + Send + Sync {
    /// Make suitable replacement
    fn generate<'a>(&self, m: &Match<'a>) -> Cow<'a, str>;
}

clone_trait_object!(Tag);

#[cfg(test)]
mod tests {
    use crate::Tag;

    // dirty hack, implementing eq in terms of Debug. this is only used for testing
    impl PartialEq for dyn Tag {
        fn eq(&self, other: &Self) -> bool {
            format!("{:?}", self) == format!("{:?}", other)
        }
    }
}
