//! **stager** - This crate stages files for packaging
//!
//! ```toml
//! [dependencies]
//! stager = "0.3"
//! ```

#![warn(warnings)]

#[macro_use]
extern crate failure;
extern crate globwalk;
#[cfg(feature = "de")]
extern crate liquid;
#[macro_use]
extern crate log;
#[cfg(feature = "de")]
#[macro_use]
extern crate serde;

pub mod action;
pub mod builder;
#[cfg(feature = "de")]
pub mod de;

mod error;
