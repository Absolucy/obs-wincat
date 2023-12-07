// SPDX-License-Identifier: MPL-2.0
use std::time::Duration;

// SPDX-License-Identifier: MPL-2.0
use crate::window::{get_processes, Process};
use ahash::AHashMap;
use ctor::ctor;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

pub static PROCESS_LIST: Lazy<RwLock<Vec<Process>>> =
	Lazy::new(|| RwLock::new(Vec::with_capacity(128)));

#[inline(never)]
pub(crate) fn process_loading_thread() {
	if let Err(err) = crate::util::set_thread_priority() {
		error!("failed to set thread priority: {:?}", err);
	}
	let mut process_map = AHashMap::<u32, Process>::with_capacity(128);
	let mut secondary_buffer = Vec::<Process>::with_capacity(128);
	loop {
		process_map.clear();
		secondary_buffer.clear();
		get_processes(&mut process_map);
		secondary_buffer.extend(process_map.drain().map(|(_, v)| v));
		std::mem::swap(&mut *PROCESS_LIST.write(), &mut secondary_buffer);
		std::thread::sleep(Duration::from_secs(1));
	}
}

#[ctor]
fn init() {
	std::thread::spawn(process_loading_thread);
}
