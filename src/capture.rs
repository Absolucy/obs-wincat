// SPDX-License-Identifier: MPL-2.0
#![allow(non_camel_case_types)]
use std::os::raw::c_void;

type winrt_capture = c_void;

#[link(name = "libobs-winrt")]
extern "C" {
	fn winrt_capture_supported() -> bool;
	fn winrt_capture_cursor_toggle_supported() -> bool;
	fn winrt_capture_init_window(
		cursor: bool,
		window: isize,
		client_area: bool,
		force_sdr: bool,
	) -> *mut winrt_capture;
	fn winrt_capture_free(capture: *mut winrt_capture);
	fn winrt_capture_active(capture: *const winrt_capture) -> bool;
	fn winrt_capture_show_cursor(capture: *mut winrt_capture, show: bool);
	fn winrt_capture_render(capture: *const winrt_capture);
	fn winrt_capture_width(capture: *const winrt_capture) -> u32;
	fn winrt_capture_height(capture: *const winrt_capture) -> u32;
}

#[repr(transparent)]
pub struct WinrtCapture {
	capture: *mut winrt_capture,
}

impl WinrtCapture {
	pub fn new(cursor: bool, window: isize, client_area: bool, force_sdr: bool) -> Option<Self> {
		unsafe {
			let capture = winrt_capture_init_window(cursor, window, client_area, force_sdr);
			if capture.is_null() {
				None
			} else {
				Some(Self { capture })
			}
		}
	}

	pub fn active(&self) -> bool {
		unsafe { winrt_capture_active(self.capture) }
	}

	pub fn show_cursor(&mut self, show: bool) {
		unsafe {
			winrt_capture_show_cursor(self.capture, show);
		}
	}

	pub fn render(&self) {
		unsafe {
			winrt_capture_render(self.capture);
		}
	}

	pub fn width(&self) -> u32 {
		unsafe { winrt_capture_width(self.capture) }
	}

	pub fn height(&self) -> u32 {
		unsafe { winrt_capture_height(self.capture) }
	}
}

impl Drop for WinrtCapture {
	fn drop(&mut self) {
		unsafe {
			winrt_capture_free(self.capture);
		}
	}
}
