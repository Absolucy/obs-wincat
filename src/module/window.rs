// SPDX-License-Identifier: MPL-2.0
use crate::window::{get_processes, Process};
use ahash::AHashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::{
	sync::atomic::{AtomicBool, Ordering},
	time::Duration,
};

pub static PROCESS_LIST: Lazy<RwLock<Vec<Process>>> =
	Lazy::new(|| RwLock::new(Vec::with_capacity(128)));
pub static SHOULD_RUN: AtomicBool = AtomicBool::new(false);

#[inline(never)]
pub(crate) fn process_loading_thread() {
	if SHOULD_RUN.swap(true, Ordering::SeqCst) {
		return;
	}
	scopeguard::defer! {
		SHOULD_RUN.store(false, Ordering::SeqCst);
	};
	std::thread::sleep(Duration::from_secs(1));
	let mut process_map = AHashMap::<u32, Process>::with_capacity(128);
	let mut secondary_buffer = Vec::<Process>::with_capacity(128);
	while SHOULD_RUN.load(Ordering::SeqCst) {
		process_map.clear();
		secondary_buffer.clear();
		get_processes(&mut process_map);
		secondary_buffer.extend(process_map.drain().map(|(_, v)| v));
		std::mem::swap(&mut *PROCESS_LIST.write(), &mut secondary_buffer);
		std::thread::sleep(Duration::from_secs(1));
	}
}
