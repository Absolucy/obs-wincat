// SPDX-License-Identifier: MPL-2.0
pub mod window;

use crate::source::CAPTURES;
use obs_wrapper::{
	log::Logger,
	module::{LoadContext, Module, ModuleContext},
	obs_sys::OBS_SOURCE_ASYNC_VIDEO,
	source::Icon,
	string::ObsString,
};

#[cfg_attr(target_pointer_width = "32", repr(align(64)))]
#[cfg_attr(target_pointer_width = "64", repr(align(128)))]
pub struct WincatModule {
	context: ModuleContext,
}

impl Module for WincatModule {
	fn new(ctx: ModuleContext) -> Self {
		Self { context: ctx }
	}

	fn get_ctx(&self) -> &ModuleContext {
		&self.context
	}

	fn load(&mut self, load_context: &mut LoadContext) -> bool {
		let _ = Logger::new().init();
		if let Err(err) = crate::hook::load_hook() {
			error!("failed to load hook: {}", err);
		}
		let mut source = load_context
			.create_source_builder::<crate::source::WincatSource>()
			.with_icon(Icon::WindowCapture)
			.enable_get_properties()
			.enable_get_defaults()
			.enable_deactivate()
			.enable_video_tick()
			.enable_get_name()
			.enable_update()
			.build();
		source.as_mut().output_flags = OBS_SOURCE_ASYNC_VIDEO;
		load_context.register_source(source);
		true
	}

	fn unload(&mut self) {
		crate::hook::force_unhook();
		CAPTURES
			.write()
			.drain()
			.filter_map(|(_, source)| source.upgrade())
			.for_each(|source| {
				source.write().take();
			});
	}

	fn description() -> ObsString {
		obs_string!("Better window capture for modern Window!")
	}

	fn name() -> ObsString {
		obs_string!("Wincat")
	}

	fn author() -> ObsString {
		obs_string!("Absolucy")
	}
}

obs_register_module!(WincatModule);
