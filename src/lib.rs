//! **stager** - This crate stages files for packaging
//!
//! ## Install
//!
//! ```toml
//! [dependencies]
//! stager = "0.3"
//! ```
//!
//! ## Example
//!
//! [staging][staging] will
//! - Read a stage configuration (using `staging::de`) and variables to be substitued using [liquid][liquid].
//! - Transform the configuration and variables into the stager API (`staging::builder`).
//! - Transform the builders into distinct actions to be performed on the file system (`staging::action`).
//! - Apply these actions to the target directory.
//!
//! [staging]: https://github.com/crate-ci/stager/blob/master/src/bin/staging/main.rs
//! [liquid]: https://shopify.github.io/liquid/
//!
//! ### Packaging Systems
//!
//! - [`cargo-tarball`][tarball]: Tarball a Rust projct for github releases.
//!
//! [tarball]: https://github.com/crate-ci/cargo-tarball

#![warn(missing_docs, missing_debug_implementations)]

extern crate failure;
extern crate globwalk;
#[cfg(feature = "de")]
extern crate liquid;
#[macro_use]
extern crate log;
#[cfg(feature = "de")]
#[macro_use]
extern crate serde;
extern crate walkdir;

pub mod action;
pub mod builder;
#[cfg(feature = "de")]
pub mod de;
#[cfg(feature = "de")]
mod template;

mod error;
