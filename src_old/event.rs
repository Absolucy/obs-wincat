// SPDX-License-Identifier: MPL-2.0
use std::{
	sync::{Arc, Weak},
	time::Duration,
};
use windows::Win32::{
	Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
	System::Threading::{CreateEventA, ResetEvent, SetEvent, WaitForSingleObject},
};

#[cfg_attr(target_pointer_width = "32", repr(align(64)))]
#[cfg_attr(target_pointer_width = "64", repr(align(128)))]
#[derive(Clone)]
pub enum EventHolder {
	Strong(Arc<EventHandle>),
	Weak(Weak<EventHandle>),
}

impl EventHolder {
	pub fn new() -> Option<Self> {
		let event = unsafe { CreateEventA(None, false, None, None) }.ok()?;
		Some(Self::Strong(Arc::new(EventHandle(event))))
	}

	pub fn signal(&self) -> bool {
		match self {
			Self::Strong(handle) => unsafe { SetEvent(handle.0) }.is_ok(),
			Self::Weak(handle) => match handle.upgrade() {
				Some(handle) => unsafe { SetEvent(handle.0) }.is_ok(),
				None => false,
			},
		}
	}

	pub fn reset(&self) -> bool {
		match self {
			Self::Strong(handle) => unsafe { ResetEvent(handle.0) }.is_ok(),
			Self::Weak(handle) => match handle.upgrade() {
				Some(handle) => unsafe { ResetEvent(handle.0) }.is_ok(),
				None => false,
			},
		}
	}

	pub fn weaken(&self) -> Self {
		match self {
			Self::Strong(handle) => Self::Weak(Arc::downgrade(handle)),
			Self::Weak(handle) => Self::Weak(handle.clone()),
		}
	}

	pub fn listener(&self) -> EventListener {
		match self {
			Self::Strong(handle) => EventListener(Arc::downgrade(handle)),
			Self::Weak(handle) => EventListener(handle.clone()),
		}
	}
}

#[cfg_attr(target_pointer_width = "32", repr(align(64)))]
#[cfg_attr(target_pointer_width = "64", repr(align(128)))]
pub struct EventListener(Weak<EventHandle>);

impl EventListener {
	pub fn check(&self) -> Option<bool> {
		self.wait_impl(0)
	}

	pub fn wait(&self, timeout: impl Into<Option<Duration>>) -> Option<bool> {
		let timeout = timeout
			.into()
			.map(|timeout| timeout.as_millis() as u32)
			.unwrap_or(u32::MAX);
		self.wait_impl(timeout)
	}

	fn wait_impl(&self, timeout: u32) -> Option<bool> {
		let handle = match self.0.upgrade() {
			Some(handle) => handle,
			None => return None,
		};
		match unsafe { WaitForSingleObject(handle.0, timeout) } {
			WAIT_OBJECT_0 => Some(true),
			WAIT_TIMEOUT => Some(false),
			_ => None,
		}
	}
}

#[repr(transparent)]
pub struct EventHandle(HANDLE);

unsafe impl Send for EventHandle {}
unsafe impl Sync for EventHandle {}

impl Drop for EventHandle {
	fn drop(&mut self) {
		unsafe { CloseHandle(self.0) }.ok();
	}
}
