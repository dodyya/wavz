use std::fmt::Debug;
use std::time::Instant;

use bytemuck::cast_slice;

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::BoxSlice2D;
use crate::fft::RESOLUTION;
use crate::fft::fft_spectrum_into;
use crate::rgba::Rgba;

// TODO: lower the scope of some of these constants (move them into functions or structs if not used everywhere)
// TODO: Allow thin-screen playback by revamping playback
// to have a scrolling vertical bar that represents "now".
// Ought to defer until we have audio.
const PIXEL_SCALE: usize = 2;
const MAX_WIDTH: usize = 1500; // Maximum screen width, determines playability
const RGBA: usize = 4; // Magic number for bytes/color
const INERTIA_RATIO: f32 = 5f32 / 6f32; // bigger number => more inertia

struct PlayState {
	pub x_offset: usize,
	pub scroll_v: f32,
	pub playing: Option<(Instant, usize)>,
	pub ffts_per_second: u32,
}

impl PlayState {
	fn apply_inertia(inertia: f32, delta: f32) -> f32 {
		(INERTIA_RATIO) * inertia + (1f32 - INERTIA_RATIO) * delta
	}

	fn inc(&mut self) {
		if let Some((start_time, start_x)) = self.playing {
			let dur = start_time.elapsed();
			self.x_offset = start_x + (self.ffts_per_second as f32 * dur.as_secs_f32()) as usize;
		}
	}

	fn stop(&mut self) {
		if let Some((start_time, start_x)) = self.playing {
			let dur = start_time.elapsed();
			self.x_offset = start_x + (self.ffts_per_second as f32 * dur.as_secs_f32()) as usize;
		}
		self.playing = None;
	}

	fn tog(&mut self) {
		if let Some((start_time, start_x)) = self.playing {
			let dur = start_time.elapsed();
			self.x_offset = start_x + (self.ffts_per_second as f32 * dur.as_secs_f32()) as usize;
			self.playing = None;
		} else {
			self.playing = Some((Instant::now(), self.x_offset));
		}
	}

	fn handle_scroll(&mut self, scroll_in: f32, dom: isize, width: isize) {
		let scroll_out = Self::apply_inertia(self.scroll_v, scroll_in) as isize;
		match scroll_out {
			(1..) => {
				let new_pos = self.x_offset as isize + scroll_out;
				if new_pos < dom - width {
					self.x_offset = new_pos as usize;
				}
				self.stop();
			},
			(..=-1) => {
				let new_pos = self.x_offset as isize + scroll_out;
				if new_pos > 0 {
					self.x_offset = new_pos as usize;
				}
				self.stop();
			},
			_ => {},
		}
		self.scroll_v = Self::apply_inertia(self.scroll_v, scroll_out as f32);
	}
}

impl Debug for PlayState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} samples in, ", self.x_offset)?;
		write!(
			f,
			"{}",
			if self.playing.is_some() { "Playing, " } else { "Paused, " }
		)?;
		let start_t = self.x_offset as f64 / self.ffts_per_second as f64;
		let mins = (start_t / 60.0).floor();
		if mins > 0.0 {
			write!(f, "t={}:{:.2}", mins, start_t % 60.0)?;
		} else {
			write!(f, "t={:.2}", start_t % 60.0)?;
		}
		Ok(())
	}
}

pub fn show_spectrogram(spectra: BoxSlice2D<Rgba>, ffts_per_second: u32) {
	let domain = spectra.width;
	let range = spectra.height;
	let img = spectra.data;

	let event_loop = EventLoop::new().unwrap();
	let mut input = WinitInputHelper::new();
	let mut play: Option<PlayState> = (domain > MAX_WIDTH).then_some(PlayState {
		x_offset: 0,
		scroll_v: 0.0,
		playing: None,
		ffts_per_second,
	});
	let height = range;
	let width = domain.clamp(0, MAX_WIDTH);

	let window = {
		let size = PhysicalSize::new((width * PIXEL_SCALE) as u32, (height * PIXEL_SCALE) as u32);
		WindowBuilder::new()
			.with_title("")
			.with_inner_size(size)
			.with_min_inner_size(size)
			.with_resizable(false)
			.build(&event_loop)
			.unwrap()
	};

	let mut pixels = {
		let window_size = window.inner_size();
		let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
		Pixels::new(width as u32, height as u32, surface_texture).unwrap()
	};

	let _ = event_loop.run(|event, elwt| {
		if let Event::WindowEvent {
			event: WindowEvent::RedrawRequested,
			..
		} = event
		{
			let frame = pixels.frame_mut();
			if let Some(ps) = play.as_mut() {
				for y in 0..range {
					//Drawing the horizontal subview.
					frame[RGBA * width * y..RGBA * width * (y + 1)].copy_from_slice(cast_slice(
						&img[(ps.x_offset + y * domain)..(ps.x_offset + y * domain + width)],
					));
				}
				ps.inc();
			} else {
				frame.copy_from_slice(cast_slice(&img));
			}

			if pixels.render().is_err() {
				elwt.exit();
				return;
			}
		}

		if input.update(&event) {
			if input.key_pressed(KeyCode::KeyQ) || input.close_requested() {
				elwt.exit();
				return;
			}

			if let Some(ps) = play.as_mut() {
				ps.handle_scroll(input.scroll_diff().1, domain as isize, width as isize);

				if input.key_pressed(KeyCode::Space) {
					ps.tog();
				}

				window.set_title(&format!("{domain} samples generated. {ps:?}"));
			}

			window.request_redraw();
		}
	});
}

pub fn sliding_spectra(samples: Box<[i16]>, step_size: usize) -> BoxSlice2D<f32> {
	let num_ffts = (samples.len() - RESOLUTION) / step_size;
	let mut start = 0;
	let mut out = BoxSlice2D::<f32>::new(RESOLUTION / 2, num_ffts);

	for i in 0..num_ffts {
		let mut fr = Box::new([0.0; RESOLUTION]);

		for j in 0..RESOLUTION {
			fr[j] = samples[j + start] as f32 / i16::MAX as f32 / 2.0;
		}

		fft_spectrum_into(out.row_mut(i), fr.as_mut());
		start += step_size;
	}

	out
}
