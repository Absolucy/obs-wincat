// SPDX-License-Identifier: MPL-2.0
use crate::{
	capture::{Capture, CaptureHandler},
	context::Source,
	lua::WeakLuaContext,
	source::CaptureOptions,
	window::Window,
};
use anyhow::{Context, Result};
use crossbeam_channel::{select, Receiver, Sender};
use ctor::{ctor, dtor};
use mlua::{Function, LuaSerdeExt, Value};
use parking_lot::RwLock;
use slotmap::{DefaultKey, SlotMap};
use std::sync::{
	atomic::{AtomicBool, AtomicUsize, Ordering},
	Arc, Weak,
};
use windows::Win32::{
	Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
	System::SystemServices::IMAGE_DOS_HEADER,
	UI::WindowsAndMessaging::{
		CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HCBT_CREATEWND, HCBT_DESTROYWND,
		HHOOK, WH_CBT,
	},
};
use windows_capture::{
	capture::WindowsCaptureHandler,
	settings::{ColorFormat as CaptureColorFormat, Settings as CaptureSettings},
	window::Window as CaptureWindow,
};

pub type CallbackSender = Option<Sender<Option<isize>>>;

static HOOK_COUNT: AtomicUsize = AtomicUsize::new(0);

extern "C" {
	#[link_name = "__ImageBase"]
	static IMAGE_BASE: IMAGE_DOS_HEADER;
}

#[dynamic]
pub(crate) static EVENT_CHANNEL: (Sender<Option<isize>>, Receiver<Option<isize>>) =
	crossbeam_channel::bounded(16);
#[dynamic(drop)]
pub static mut HOOK_LISTENERS: SlotMap<DefaultKey, Sender<Option<isize>>> =
	SlotMap::with_capacity(8);
#[dynamic]
static mut HOOK_ID: Option<HHOOK> = None;

#[ctor]
fn hook_distributor() {
	std::thread::spawn(|| {
		if let Err(err) = crate::util::set_thread_priority() {
			error!("failed to set thread priority: {:?}", err);
		}
		while let Ok(val) = EVENT_CHANNEL.1.recv() {
			for tx in HOOK_LISTENERS.read().values() {
				tx.send(val).ok();
			}
		}
	});
}

#[dtor]
fn unhook() {
	force_unhook();
}

pub(crate) fn force_unhook() {
	HOOK_COUNT.store(0, Ordering::SeqCst);
	if let Some(hook) = HOOK_ID.write().take() {
		unsafe { UnhookWindowsHookEx(hook).ok() };
	}
	HOOK_LISTENERS.write().clear();
}

unsafe extern "system" fn hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
	if matches!(n_code as u32, HCBT_CREATEWND | HCBT_DESTROYWND) {
		EVENT_CHANNEL.0.send(Some(w_param.0 as isize)).ok();
	}

	CallNextHookEx(HHOOK::default(), n_code, w_param, l_param)
}

pub(crate) fn load_hook() -> Result<()> {
	/*unsafe {
		if HOOK_COUNT.fetch_add(1, Ordering::SeqCst) > 0 {
			warn!("attempted to set window hook, but it's already loaded...");
			return Ok(());
		}
		let instance = HINSTANCE(&IMAGE_BASE as *const _ as isize);
		let hook = SetWindowsHookExW(WH_CBT, Some(hook_proc), instance, 0)
			.context("failed to setup window hook")?;
		*HOOK_ID.write() = Some(hook);
	}
	info!("window hook loaded");*/
	Ok(())
}

pub(crate) fn unload_hook() -> Result<()> {
	if HOOK_COUNT.fetch_sub(1, Ordering::SeqCst) > 1 {
		return Ok(());
	}
	let hook = match HOOK_ID.write().take() {
		Some(hook) => hook,
		None => {
			warn!("attempted to unset window hook, but there isn't one anyways...");
			return Ok(());
		}
	};
	unsafe { UnhookWindowsHookEx(hook).context("failed to unhook")? }
	info!("window hook unloaded");
	Ok(())
}

pub fn setup_lua_callback_hook(
	src: Arc<Source>,
	lua: WeakLuaContext,
	window: Weak<RwLock<Option<CaptureHandler>>>,
	options: Weak<RwLock<CaptureOptions>>,
	cancel_rx: Receiver<()>,
) -> Sender<Option<isize>> {
	let (tx, rx) = crossbeam_channel::unbounded::<Option<isize>>();
	let rtx = tx.clone();
	std::thread::spawn(move || {
		if let Err(err) = crate::util::set_thread_priority() {
			error!("failed to set thread priority: {:?}", err);
		}
		let hook_slot = HOOK_LISTENERS.write().insert(tx);
		scopeguard::defer! {
			HOOK_LISTENERS.write().remove(hook_slot);
		}
		lua_callback_hook(src, lua, rx, cancel_rx, window, options);
	});
	rtx.try_send(None).ok();
	rtx
}

#[inline(never)]
fn lua_callback_hook(
	src: Arc<Source>,
	lua: WeakLuaContext,
	rx: Receiver<Option<isize>>,
	cancel_rx: Receiver<()>,
	capture: Weak<RwLock<Option<CaptureHandler>>>,
	options: Weak<RwLock<CaptureOptions>>,
) {
	loop {
		select! {
			recv(rx) -> incoming_hwnd => match incoming_hwnd {
				Ok(incoming_hwnd) => {
					if !lua_callback_inner(&src, &lua, incoming_hwnd, &capture, &options) {
						break
					}
				}
				_ => break,
			},
			recv(cancel_rx) -> _ => break,
			default => std::thread::yield_now(),
		}
	}
}

fn lua_callback_inner(
	src: &Arc<Source>,
	lua: &WeakLuaContext,
	incoming_hwnd: Option<isize>,
	capture: &Weak<RwLock<Option<CaptureHandler>>>,
	options: &Weak<RwLock<CaptureOptions>>,
) -> bool {
	let (lua, capture) = match (lua.upgrade(), capture.upgrade()) {
		(Some(lua), Some(capture)) => (lua, capture),
		_ => return false,
	};
	let lua = lua.lock();
	let handler: Function = lua
		.named_registry_value("select_window")
		.expect("failed to get select_window");
	let procs = lua
		.to_value(&*crate::module::window::PROCESS_LIST.read())
		.expect("failed to serialize processes");

	let hwnd = match handler.call::<_, Option<Value>>(procs) {
		Ok(Some(window_value)) => {
			let window = match lua.from_value::<Window>(window_value) {
				Ok(window) => window,
				Err(err) => {
					error!("failed to deserialize window: {:?}", err);
					*capture.write() = None;
					return false;
				}
			};
			let current = capture.read();
			if !crate::util::force_capture_refresh(current.as_ref(), incoming_hwnd, window.hwnd) {
				return true;
			}
			window.hwnd
		}
		Ok(None) => {
			info!("selecting window: None");
			stop_capture(&mut capture.write());
			return true;
		}
		Err(err) => {
			error!("failed to call select_window: {:?}", err);
			return false;
		}
	};
	let pid = match crate::util::hwnd_to_pid(hwnd) {
		Some(pid) => pid,
		None => return true,
	};
	let window = CaptureWindow::from_raw_hwnd(HWND(hwnd));
	let options = match options.upgrade() {
		Some(options) => options,
		None => return false,
	};
	let options = options.read();
	let settings = match CaptureSettings::new(
		window,
		Some(options.cursor),
		Some(options.border),
		CaptureColorFormat::Bgra8,
		src.clone(),
	) {
		Ok(settings) => settings,
		Err(err) => {
			error!("failed to setup capture: {:?}", err);
			return false;
		}
	};
	let mut capture_handle = capture.write();
	stop_capture(&mut capture_handle);
	info!("restarting capture");
	let control = Capture::start_free_threaded(settings).expect("failed to start capture");
	*capture_handle = Some(CaptureHandler {
		control,
		hwnd,
		pid,
		force_update: AtomicBool::new(false),
	});
	true
}

pub(crate) fn stop_capture(handler: &mut Option<CaptureHandler>) {
	if let Some(capture) = handler.take() {
		if let Err(err) = capture.control.stop() {
			error!("failed to stop capture: {:?}", err);
		}
	}
}
