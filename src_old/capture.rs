// SPDX-License-Identifier: MPL-2.0
use crate::context::Source;
use anyhow::{Context, Error, Result};
use obs_wrapper::obs_sys::{
	obs_source_frame, obs_source_output_video, video_format_VIDEO_FORMAT_BGRA,
};
use std::{
	sync::{
		atomic::{AtomicBool, AtomicUsize, Ordering},
		Arc,
	},
	time::{SystemTime, UNIX_EPOCH},
};
use windows_capture::{
	capture::{CaptureControl, WindowsCaptureHandler},
	frame::Frame,
	graphics_capture_api::InternalCaptureControl,
};

static CAPTURE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn count() -> usize {
	CAPTURE_COUNT.load(Ordering::SeqCst)
}

pub struct CaptureHandler {
	pub(crate) control: CaptureControl<Capture, Error>,
	pub(crate) hwnd: isize,
	pub(crate) pid: u32,
	pub(crate) force_update: AtomicBool,
}

#[cfg_attr(target_pointer_width = "32", repr(align(64)))]
#[cfg_attr(target_pointer_width = "64", repr(align(128)))]
pub struct Capture {
	context: Arc<Source>,
}

impl WindowsCaptureHandler for Capture {
	type Error = Error;
	type Flags = Arc<Source>;

	fn new(context: Self::Flags) -> Result<Self> {
		CAPTURE_COUNT.fetch_add(1, Ordering::SeqCst);
		Ok(Self { context })
	}

	fn on_frame_arrived(
		&mut self,
		frame: &mut Frame,
		_control: InternalCaptureControl,
	) -> Result<()> {
		let mut buffer = frame.buffer().context("failed to get framebuffer")?;
		let width = buffer.width();
		let height = buffer.height();
		let timestamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap_or_else(|_| unreachable!("time went backwards"))
			.as_millis() as u64;
		let mut frame = obs_source_frame {
			width,
			height,
			timestamp,
			format: video_format_VIDEO_FORMAT_BGRA,
			..obs_source_frame::default()
		};
		frame.linesize[0] = width * 4;
		frame.data[0] = buffer
			.as_raw_nopadding_buffer()
			.context("failed to get buffer without paddin")?
			.as_mut_ptr();
		unsafe { obs_source_output_video(self.context.obs_context(), &frame) }
		Ok(())
	}

	fn on_closed(&mut self) -> Result<()> {
		CAPTURE_COUNT.fetch_sub(1, Ordering::SeqCst);
		Ok(())
	}
}
