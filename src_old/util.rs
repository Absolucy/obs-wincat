// SPDX-License-Identifier: MPL-2.0
use crate::{capture::CaptureHandler, lua::LuaContext};
use anyhow::{anyhow, Result};
use mlua::Function;
use obs_wrapper::{data::DataObj, obs_sys::obs_source_t, source::SourceContext, string::ObsString};
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

// OH GOD WHY
pub(crate) unsafe fn fuck(context: &SourceContext) -> *mut obs_source_t {
	struct _SourceContext {
		inner: *mut obs_source_t,
	}
	let context = context as *const _ as *mut _SourceContext;
	(*context).inner
}

pub(crate) fn load_script(lua_context: &LuaContext, settings: &DataObj) {
	let lua = lua_context.lock();
	let script = match settings.get::<ObsString>(obs_string!("script")) {
		Some(script) if !script.as_str().trim().is_empty() => script,
		_ => {
			if let Err(err) = lua.unset_named_registry_value("select_window") {
				error!("Failed to unset select_window: {:?}", err);
			}
			return;
		}
	};
	match lua.load(script.as_str()).eval::<Function>() {
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

pub(crate) fn force_capture_refresh(
	capture: Option<&CaptureHandler>,
	incoming_hwnd: Option<isize>,
	selected_hwnd: isize,
) -> bool {
	let capture = match capture {
		Some(capture) => capture,
		None => return true,
	};
	if selected_hwnd != capture.hwnd {
		return true;
	}
	if capture
		.force_update
		.swap(false, std::sync::atomic::Ordering::SeqCst)
	{
		return true;
	}
	if let Some(incoming_hwnd) = incoming_hwnd {
		if incoming_hwnd == selected_hwnd || hwnd_to_pid(incoming_hwnd) != Some(capture.pid) {
			return false;
		}
		return crate::window::is_window_visible(HWND(incoming_hwnd))
			&& !crate::window::get_window_title(HWND(incoming_hwnd)).is_empty();
	}
	false
}
