// SPDX-License-Identifier: MPL-2.0
pub mod window;

use std::sync::atomic::Ordering;

use obs_wrapper::{
	log::Logger,
	module::{LoadContext, Module, ModuleContext},
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
		let source = load_context
			.create_source_builder::<crate::source::WincatSource>()
			.with_icon(Icon::WindowCapture)
			.enable_get_properties()
			.enable_get_defaults()
			.enable_deactivate()
			.enable_video_render()
			.enable_video_tick()
			.enable_get_height()
			.enable_get_width()
			.enable_get_name()
			.enable_update()
			.build();
		load_context.register_source(source);
		std::thread::spawn(window::process_loading_thread);
		true
	}

	fn unload(&mut self) {
		window::SHOULD_RUN.store(false, Ordering::SeqCst);
	}

	fn description() -> ObsString {
		obs_string!("Better window capture for modern Windows!")
	}

	fn name() -> ObsString {
		obs_string!("Wincat")
	}

	fn author() -> ObsString {
		obs_string!("Absolucy")
	}
}

obs_register_module!(WincatModule);
