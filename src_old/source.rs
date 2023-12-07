// SPDX-License-Identifier: MPL-2.0
use crate::{capture::CaptureHandler, context::Source, hook::CallbackSender, lua::LuaContext};
use crossbeam_channel::{Receiver, Sender};
use obs_wrapper::{
	data::DataObj,
	properties::{BoolProp, Properties, TextProp, TextType},
	source::{
		ActivateSource, CreatableSourceContext, DeactivateSource, GetDefaultsSource, GetNameSource,
		GetPropertiesSource, GlobalContext, SourceContext, SourceType, Sourceable, UpdateSource,
		VideoTickSource,
	},
	string::ObsString,
};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use slotmap::{DefaultKey, SlotMap};
use std::sync::{atomic::Ordering, Arc, Weak};

#[dynamic(drop)]
pub static mut CAPTURES: SlotMap<DefaultKey, Weak<RwLock<Option<CaptureHandler>>>> =
	SlotMap::with_capacity(8);

#[cfg_attr(target_pointer_width = "32", repr(align(64)))]
#[cfg_attr(target_pointer_width = "64", repr(align(128)))]
pub struct WincatSource {
	source: Arc<Source>,
	lua_context: LuaContext,
	options: Arc<RwLock<CaptureOptions>>,
	capture: Arc<RwLock<Option<CaptureHandler>>>,
	capture_id: DefaultKey,
	halt_callback: (Sender<()>, Receiver<()>),
	callback_event: RwLock<CallbackSender>,
	ticks: f32,
}

impl WincatSource {
	pub fn stop(&self) {
		self.halt_callback.0.try_send(()).ok();
		crate::hook::stop_capture(&mut self.capture.write())
	}

	pub fn run_callback(&self) {
		let callback = self.callback_event.upgradable_read();
		if callback
			.as_ref()
			.and_then(|tx| tx.send(None).ok())
			.is_none()
		{
			self.create_callback(&mut RwLockUpgradableReadGuard::upgrade(callback));
		}
	}

	fn create_callback(&self, callback: &mut CallbackSender) {
		let new_callback = crate::hook::setup_lua_callback_hook(
			self.source.clone(),
			Arc::downgrade(&self.lua_context),
			Arc::downgrade(&self.capture),
			Arc::downgrade(&self.options),
			self.halt_callback.1.clone(),
		);
		new_callback.try_send(None).ok();
		*callback = Some(new_callback);
	}

	fn update_options(&self, settings: &DataObj) -> bool {
		let new_options = CaptureOptions {
			cursor: settings.get::<bool>(obs_string!("cursor")).unwrap_or(true),
			border: settings
				.get::<bool>(obs_string!("borders"))
				.unwrap_or(false),
		};
		let current_options = self.options.upgradable_read();
		if *current_options == new_options {
			false
		} else {
			*RwLockUpgradableReadGuard::upgrade(current_options) = new_options;
			if let Some(CaptureHandler { force_update, .. }) = self.capture.read().as_ref() {
				force_update.store(true, Ordering::SeqCst);
			}
			true
		}
	}
}

impl Sourceable for WincatSource {
	fn get_id() -> ObsString {
		obs_string!("wincat")
	}

	fn get_type() -> SourceType {
		SourceType::INPUT
	}

	fn create(create: &mut CreatableSourceContext<Self>, source: SourceContext) -> Self {
		let source = Arc::new(Source::from(source));
		let lua_context = crate::lua::setup_luau_context();
		let options = Arc::new(RwLock::new(CaptureOptions::new_from_data(&create.settings)));
		let capture = Arc::new(RwLock::new(None));
		let capture_id = CAPTURES.write().insert(Arc::downgrade(&capture));
		let halt_callback = crossbeam_channel::bounded::<()>(1);
		crate::util::load_script(&lua_context, &create.settings);
		Self {
			source,
			lua_context,
			options,
			capture,
			capture_id,
			halt_callback,
			callback_event: RwLock::new(None),
			ticks: 0.0,
		}
	}
}

impl GetNameSource for WincatSource {
	fn get_name() -> ObsString {
		obs_string!("Advanced Window Capture (WGC)")
	}
}

impl GetPropertiesSource for WincatSource {
	fn get_properties(&mut self) -> Properties {
		let mut props = Properties::new();
		props.add(
			obs_string!("script"),
			obs_string!("Selector Script (Luau)"),
			TextProp::new(TextType::Multiline),
		);
		props.add(
			obs_string!("cursor"),
			obs_string!("Capture Cursor"),
			BoolProp,
		);
		props.add(
			obs_string!("borders"),
			obs_string!("Draw Borders"),
			BoolProp,
		);
		props
	}
}

impl UpdateSource for WincatSource {
	fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
		self.update_options(settings);
		crate::util::load_script(&self.lua_context, settings);
		self.run_callback();
	}
}

impl GetDefaultsSource for WincatSource {
	fn get_defaults(settings: &mut DataObj) {
		settings.set_default::<ObsString>(obs_string!("script"), obs_string!(""));
		settings.set_default::<bool>(obs_string!("cursor"), true);
		settings.set_default::<bool>(obs_string!("borders"), false);
	}
}

impl ActivateSource for WincatSource {
	fn activate(&mut self) {
		self.run_callback();
	}
}

impl DeactivateSource for WincatSource {
	fn deactivate(&mut self) {
		self.stop();
	}
}

impl VideoTickSource for WincatSource {
	fn video_tick(&mut self, seconds: f32) {
		self.ticks += seconds;
		if self.ticks >= 1.5 {
			self.run_callback();
			self.ticks = 0.0;
		}
	}
}

impl Drop for WincatSource {
	fn drop(&mut self) {
		CAPTURES.write().remove(self.capture_id);
		self.stop();
	}
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CaptureOptions {
	pub cursor: bool,
	pub border: bool,
}

impl CaptureOptions {
	pub fn new_from_data(settings: &DataObj) -> Self {
		Self {
			cursor: settings.get::<bool>(obs_string!("cursor")).unwrap_or(true),
			border: settings
				.get::<bool>(obs_string!("borders"))
				.unwrap_or(false),
		}
	}
}

impl Default for CaptureOptions {
	fn default() -> Self {
		Self {
			cursor: true,
			border: false,
		}
	}
}
