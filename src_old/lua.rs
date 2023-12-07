// SPDX-License-Identifier: MPL-2.0
use mlua::Lua;
use parking_lot::Mutex;
use std::sync::{Arc, Weak};

pub type LuaContext = Arc<Mutex<Lua>>;
pub type WeakLuaContext = Weak<Mutex<Lua>>;

#[cold]
pub(crate) fn setup_luau_context() -> LuaContext {
	let context = Lua::new();
	context.enable_jit(true);
	if let Err(err) = context
		.globals()
		.set("print", mlua::Function::wrap(lua_print))
	{
		error!("failed to setup print() logger: {}", err);
	}
	if let Err(err) = context.sandbox(true) {
		error!("failed to sandbox lua: {}", err);
	}
	Arc::new(Mutex::new(context))
}

fn lua_print(_lua: &Lua, input: String) -> mlua::Result<()> {
	info!("{input}");
	Ok(())
}
