// SPDX-License-Identifier: MPL-2.0
use crate::{capture::WinrtCapture, window::Window};
use anyhow::{Context, Result};
use mlua::{Function, Lua, LuaSerdeExt, Value};
use obs_wrapper::{
	data::DataObj,
	properties::{BoolProp, Properties, TextProp, TextType},
	source::{
		ActivateSource, CreatableSourceContext, DeactivateSource, GetDefaultsSource,
		GetHeightSource, GetNameSource, GetPropertiesSource, GetWidthSource, GlobalContext,
		SourceContext, SourceType, Sourceable, UpdateSource, VideoRenderContext, VideoRenderSource,
		VideoTickSource,
	},
	string::ObsString,
};
use windows::Win32::Foundation::HWND;

struct CaptureSession {
	hwnd: HWND,
	title: String,
	capture: WinrtCapture,
}

#[cfg_attr(target_pointer_width = "32", repr(align(64)))]
#[cfg_attr(target_pointer_width = "64", repr(align(128)))]
pub struct WincatSource {
	lua: Lua,
	settings: Settings,
	ticks: f32,
	capture: Option<CaptureSession>,
}

#[link(name = "obs")]
extern "C" {
	fn obs_enter_graphics();
	fn obs_leave_graphics();
}

impl WincatSource {
	pub fn run_callbacks(&mut self) -> Result<()> {
		unsafe { obs_enter_graphics() };
		scopeguard::defer! { unsafe { obs_leave_graphics(); } };
		let handler: Function = self
			.lua
			.named_registry_value("select_window")
			.context("failed to get select_window")?;
		let procs = self
			.lua
			.to_value(&*crate::module::window::PROCESS_LIST.read())
			.expect("failed to serialize processes");
		std::mem::drop(self.capture.take());
		let window = match handler
			.call::<_, Option<Value>>(procs)
			.context("failed to call select_window")?
		{
			Some(window_value) => self
				.lua
				.from_value::<Window>(window_value)
				.context("failed to deserialize window")?,
			None => {
				debug!("selecting window: None");
				return Ok(());
			}
		};
		self.capture = WinrtCapture::new(
			self.settings.cursor,
			window.hwnd,
			self.settings.client_area,
			self.settings.force_sdr,
		)
		.map(|capture| CaptureSession {
			hwnd: HWND(window.hwnd),
			title: window.title,
			capture,
		});
		Ok(())
	}
}

impl Sourceable for WincatSource {
	fn get_id() -> ObsString {
		obs_string!("wincat")
	}

	fn get_type() -> SourceType {
		SourceType::INPUT
	}

	fn create(create: &mut CreatableSourceContext<Self>, _source: SourceContext) -> Self {
		let lua = crate::lua::setup_luau_context();
		let mut this = Self {
			lua,
			settings: Settings::default(),
			ticks: 0.0,
			capture: None,
		};
		this.update(&mut create.settings, create.global);
		this
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
			obs_string!("client_area"),
			obs_string!("Client Area"),
			BoolProp,
		);
		props.add(obs_string!("force_sdr"), obs_string!("Force SDR"), BoolProp);
		props
	}
}

impl UpdateSource for WincatSource {
	fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
		self.settings.script = settings
			.get::<ObsString>(obs_string!("script"))
			.map(|os| os.as_str().to_string())
			.unwrap_or_default();
		self.settings.cursor = settings.get::<bool>(obs_string!("cursor")).unwrap_or(true);
		self.settings.client_area = settings
			.get::<bool>(obs_string!("client_area"))
			.unwrap_or(false);
		self.settings.force_sdr = settings
			.get::<bool>(obs_string!("force_sdr"))
			.unwrap_or(false);
		crate::util::load_script(&self.lua, &self.settings.script);
		if let Err(err) = self.run_callbacks() {
			error!("failed to run callbacks: {}", err);
		}
	}
}

impl GetDefaultsSource for WincatSource {
	fn get_defaults(settings: &mut DataObj) {
		settings.set_default::<ObsString>(obs_string!("script"), obs_string!(""));
		settings.set_default::<bool>(obs_string!("cursor"), true);
		settings.set_default::<bool>(obs_string!("client_area"), false);
		settings.set_default::<bool>(obs_string!("force_sdr"), false);
	}
}

impl ActivateSource for WincatSource {
	fn activate(&mut self) {
		crate::util::load_script(&self.lua, &self.settings.script);
		if let Err(err) = self.run_callbacks() {
			error!("failed to run callbacks: {}", err);
		}
	}
}

impl DeactivateSource for WincatSource {
	fn deactivate(&mut self) {}
}

impl VideoTickSource for WincatSource {
	fn video_tick(&mut self, seconds: f32) {
		self.ticks += seconds;
		let capture = match self.capture.as_ref() {
			Some(capture) => capture,
			None => {
				if self.ticks >= 2.0 {
					self.ticks = 0.0;
					if let Err(err) = self.run_callbacks() {
						error!("failed to run callbacks: {}", err);
					}
				}
				return;
			}
		};

		if !capture.capture.active() {
			std::mem::drop(self.capture.take());
			warn!("capture inactive; running callbacks");
			if let Err(err) = self.run_callbacks() {
				error!("failed to run callbacks: {}", err);
			}
			return;
		}

		if self.ticks >= 5.0 {
			self.ticks = 0.0;
			let title = crate::window::get_window_title(capture.hwnd);
			if !title.is_empty() && title != capture.title {
				std::mem::drop(self.capture.take());
				warn!("window title changed; running callbacks");
				if let Err(err) = self.run_callbacks() {
					error!("failed to run callbacks: {}", err);
				}
			}
		}
	}
}

impl VideoRenderSource for WincatSource {
	fn video_render(&mut self, _context: &mut GlobalContext, _render: &mut VideoRenderContext) {
		if let Some(capture) = self.capture.as_ref() {
			capture.capture.render();
		}
	}
}

impl GetWidthSource for WincatSource {
	fn get_width(&mut self) -> u32 {
		match self.capture.as_ref() {
			Some(capture) => capture.capture.width(),
			None => 0,
		}
	}
}

impl GetHeightSource for WincatSource {
	fn get_height(&mut self) -> u32 {
		match self.capture.as_ref() {
			Some(capture) => capture.capture.height(),
			None => 0,
		}
	}
}

struct Settings {
	script: String,
	cursor: bool,
	client_area: bool,
	force_sdr: bool,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			script: String::new(),
			cursor: true,
			client_area: false,
			force_sdr: false,
		}
	}
}
