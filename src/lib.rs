#![warn(warnings)]

#[macro_use]
extern crate failure;
extern crate globwalk;
#[macro_use]
extern crate serde;

pub mod action;
pub mod builder;
pub mod de;
