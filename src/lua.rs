// SPDX-License-Identifier: MPL-2.0
use mlua::Lua;

#[cold]
pub(crate) fn setup_luau_context() -> Lua {
	let context = Lua::new();
	if let Err(err) = context
		.globals()
		.set("print", mlua::Function::wrap(lua_print))
	{
		error!("failed to setup print() logger: {}", err);
	}
	context
}

fn lua_print(_lua: &Lua, input: String) -> mlua::Result<()> {
	info!("{input}");
	Ok(())
}
