#![warn(warnings)]

#[macro_use]
extern crate failure;
extern crate globwalk;
#[cfg(feature = "de")]
extern crate liquid;
#[cfg(feature = "de")]
#[macro_use]
extern crate serde;

pub mod action;
pub mod builder;
#[cfg(feature = "de")]
pub mod de;
