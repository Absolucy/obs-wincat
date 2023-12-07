// SPDX-License-Identifier: MPL-2.0
#![warn(
	clippy::correctness,
	clippy::suspicious,
	clippy::complexity,
	clippy::perf,
	clippy::style
)]
#![allow(clippy::arc_with_non_send_sync, clippy::mut_from_ref)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_macros))]
#[macro_use]
extern crate obs_wrapper;
#[macro_use]
extern crate log;

pub mod capture;
pub mod lua;
pub mod module;
pub mod source;
pub mod util;
pub mod window;
