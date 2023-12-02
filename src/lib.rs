mod accent;
mod intensity;
mod rule;

// pub for bench
#[doc(hidden)]
pub mod utils;

#[cfg(feature = "deserialize")]
mod deserialize;

pub mod replacement;
pub use accent::Accent;
