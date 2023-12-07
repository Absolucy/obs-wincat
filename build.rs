// SPDX-License-Identifier: MPL-2.0
fn main() {
	println!(
		"cargo:rustc-link-search=native=D:/Code/Misc/obs-studio/build_x64/libobs/RelWithDebInfo"
	);
	println!(
		"cargo:rustc-link-search=native=D:/Code/Misc/obs-studio/build_x64/libobs-winrt/\
		 RelWithDebInfo"
	);
}
