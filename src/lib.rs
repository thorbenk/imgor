// Copyright 2017 Thorben Kroeger.
// Dual-licensed MIT and Apache 2.0 (see LICENSE files for details).

#![recursion_limit = "1024"]

extern crate chrono;
extern crate rexiv2;

#[macro_use]
extern crate itertools;

#[macro_use]
extern crate try_opt;

#[macro_use]
extern crate error_chain;

pub mod errors;
pub mod metadata;
pub mod grouping;
pub mod paths;
pub mod photo;

pub use errors::*;
pub use metadata::{extract_datetime, Metadata, write_derivedfrom};
pub use grouping::group_by_fn;
pub use paths::{common_prefix, CommonPrefix};
pub use photo::{Photo, group_photo_files};
