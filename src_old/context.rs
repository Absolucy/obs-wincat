// SPDX-License-Identifier: MPL-2.0
use crate::util::fuck;
use obs_wrapper::{obs_sys::obs_source_t, source::SourceContext};
use std::ops::Deref;

#[repr(transparent)]
pub struct Source(SourceContext);

impl Source {
	pub fn obs_context(&self) -> &mut obs_source_t {
		unsafe { &mut *fuck(&self.0) }
	}
}

impl Deref for Source {
	type Target = SourceContext;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsRef<SourceContext> for Source {
	#[inline]
	fn as_ref(&self) -> &SourceContext {
		&self.0
	}
}

impl From<SourceContext> for Source {
	#[inline]
	fn from(value: SourceContext) -> Self {
		Self(value)
	}
}

unsafe impl Send for Source {}
unsafe impl Sync for Source {}
