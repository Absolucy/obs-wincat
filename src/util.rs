// SPDX-License-Identifier: MPL-2.0
use mlua::{Function, Lua};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::GetWindowThreadProcessId};

pub(crate) trait TrimInPlace: Sized {
	/// Trims whitespace from the end of the string in place.
	fn trim_end_in_place(self) -> Self;

	/// Trims whitespace from the start of the string in place.
	fn trim_start_in_place(self) -> Self;

	/// Trims whitespace from both ends of the string in place.
	fn trim_in_place(self) -> Self;
}

impl TrimInPlace for String {
	#[inline]
	fn trim_end_in_place(mut self) -> Self {
		let trimmed_len = self.trim_end().len();
		self.truncate(trimmed_len);
		self
	}

	#[inline]
	fn trim_start_in_place(mut self) -> Self {
		let trimmed_start = self.len() - self.trim_start().len();
		self.replace_range(..trimmed_start, "");
		self
	}

	#[inline]
	fn trim_in_place(self) -> Self {
		self.trim_end_in_place().trim_start_in_place()
	}
}

pub(crate) fn load_script(lua: &Lua, script: &str) {
	if script.trim().is_empty() {
		if let Err(err) = lua.unset_named_registry_value("select_window") {
			error!("Failed to unset select_window: {:?}", err);
		}
		return;
	}
	match lua.load(script).eval::<Function>() {
		Ok(func) => {
			if let Err(err) = lua.set_named_registry_value("select_window", func) {
				error!("Failed to set select_window: {:?}", err);
			}
		}
		Err(err) => {
			error!("Failed to load script: {:?}", err);
			if let Err(err) = lua.unset_named_registry_value("select_window") {
				error!("Failed to unset select_window: {:?}", err);
			}
		}
	};
}

pub(crate) fn hwnd_to_pid(hwnd: isize) -> Option<u32> {
	let mut process_id = 0;
	match unsafe { GetWindowThreadProcessId(HWND(hwnd), Some(&mut process_id)) } {
		0 => None,
		_ => Some(process_id),
	}
}
