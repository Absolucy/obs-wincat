// SPDX-License-Identifier: MPL-2.0
use crate::util::TrimInPlace;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use windows::Win32::{
	Foundation::{BOOL, HWND, LPARAM, RECT},
	System::Diagnostics::ToolHelp::{
		CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
		TH32CS_SNAPPROCESS,
	},
	UI::WindowsAndMessaging::{
		EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextW, IsWindowVisible,
	},
};
use wtf8::Wtf8Buf;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(target_pointer_width = "32", repr(align(64)))]
#[cfg_attr(target_pointer_width = "64", repr(align(128)))]
pub struct Process {
	pub name: String,
	pub pid: u32,
	pub main: Option<Window>,
	pub windows: Vec<Window>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(target_pointer_width = "32", repr(align(64)))]
#[cfg_attr(target_pointer_width = "64", repr(align(128)))]
pub struct Window {
	pub title: String,
	pub class_name: String,
	pub hwnd: isize,
	pub visible: bool,
	pub x: i32,
	pub y: i32,
	pub width: i32,
	pub height: i32,
}

pub fn get_processes(processes: &mut AHashMap<u32, Process>) {
	let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
		.expect("failed to create toolhelp32 snapshot");

	let mut process_entry = PROCESSENTRY32W {
		dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
		..PROCESSENTRY32W::default()
	};

	if unsafe { Process32FirstW(snapshot, &mut process_entry) }.is_ok() {
		loop {
			let pid = process_entry.th32ProcessID;
			let len = process_entry
				.szExeFile
				.iter()
				.position(|&c| c == 0)
				.unwrap_or(0);
			let process_name = Wtf8Buf::from_ill_formed_utf16(&process_entry.szExeFile[..len])
				.into_string_lossy()
				.trim_in_place();
			processes.insert(pid, Process {
				name: process_name,
				pid,
				main: None,
				windows: Vec::new(),
			});

			if unsafe { Process32NextW(snapshot, &mut process_entry) }.is_err() {
				break;
			}
		}
	}

	let param = LPARAM(processes as *mut _ as isize);
	unsafe { EnumWindows(Some(enum_window), param) }.expect("failed to enumerate windows");
}

pub fn get_window_title(hwnd: HWND) -> String {
	let mut buffer = [0; 256];
	let len = unsafe { GetWindowTextW(hwnd, &mut buffer) as usize };
	if len == 0 {
		return String::new();
	}
	Wtf8Buf::from_ill_formed_utf16(&buffer[..len])
		.into_string_lossy()
		.trim_in_place()
}

pub fn is_window_visible(hwnd: HWND) -> bool {
	unsafe { IsWindowVisible(hwnd) }.as_bool()
}

unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
	let process_id = match crate::util::hwnd_to_pid(hwnd.0) {
		Some(pid) => pid,
		None => return BOOL(1), // Continue enumeration
	};
	let process_map = &mut *(lparam.0 as *mut AHashMap<u32, Process>);
	if let Some(process) = process_map.get_mut(&process_id) {
		let title = get_window_title(hwnd);
		if title.is_empty() {
			return BOOL(1); // Continue enumeration
		}
		let mut rect = RECT::default();
		if GetWindowRect(hwnd, &mut rect).is_err() {
			rect = RECT {
				left: i32::MIN,
				top: i32::MIN,
				right: i32::MIN,
				bottom: i32::MIN,
			};
		}
		let class_name = {
			let mut buffer = [0; 256];
			let len = GetClassNameW(hwnd, &mut buffer) as usize;
			Wtf8Buf::from_ill_formed_utf16(&buffer[..len])
				.into_string_lossy()
				.trim_in_place()
		};
		let window = Window {
			title,
			class_name,
			hwnd: hwnd.0,
			visible: is_window_visible(hwnd),
			x: rect.left,
			y: rect.top,
			width: rect.right - rect.left,
			height: rect.bottom - rect.top,
		};

		if window.visible && process.main.is_none() {
			process.main = Some(window.clone());
		}

		process.windows.push(window);
	}

	BOOL(1) // Continue enumeration
}
