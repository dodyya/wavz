#![forbid(unsafe_code)]

use std::fmt::Debug;
use std::time::Instant;

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::Float;

// TODO: lower the scope of some of these constants (move them into functions or structs if not used everywhere)
// TODO: Allow thin-screen playback by revamping playback
// to have a scrolling vertical bar that represents "now".
// Ought to defer until we have audio.
const PIXEL_SCALE: usize = 2;
const MAX_WIDTH: usize = 1500; // Maximum screen width, determines playability
const RGBA: usize = 4; // Magic number for bytes/color
const INERTIA_RATIO: f32 = 5f32 / 6f32; // bigger number => more inertia
const CUTOFF: f32 = 0.05; // Visual cutoff for what is black
const CLAMP_FACTOR: f32 = 1.0; //Twiddle this to make loud things more uniform

// TODO: Make PlayState aware of when the samples will end, so it can pause gracefully.
struct PlayState {
	pub x_offset: usize,
	pub scroll_v: f32,
	pub playing: Option<(Instant, usize)>,
	pub ffts_per_second: u32,
}

impl PlayState {
	#[inline]
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

#[inline]
// TODO: figure out if theres an easier way
/// made because `f32` doesn't implement `Ord`, so can't just use the max or min methods
fn extrema<'a>(v: impl Iterator<Item = &'a f32>) -> (f32, f32) {
	v.fold((f32::MAX, f32::MIN), |(curr_min, curr_max), &x| {
		(curr_min.min(x), curr_max.max(x))
	})
}

// TODO: switch away from nested vec arguments across the codebase. This could be moving
// towards boxed slices which can be converted into &mut [T] to take &mut [&mut T] arguments,
// or it could be moving to a custom BoxSlice2d and Slice2d struct (I think this is likely to work out best)
pub fn gen_spectrogram(spectra: &mut Vec<Vec<Float>>) -> (usize, usize, Vec<u8>) {
	// TODO: this is only ever to be used in rgb_from_hue. inline it
	// TODO: move to Rgba struct return type and more broadly across the codebase
	// TODO: make the hue argument an integer type; f32: [0.0, 360.0] doesn't really
	// make that much sense for the argument here
	#[inline]
	fn hsv2rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
		let c = v * s;
		let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
		let m = v - c;
		let (r, g, b) = match h {
			h if h < 60.0 => (c, x, 0.0),
			h if h < 120.0 => (x, c, 0.0),
			h if h < 180.0 => (0.0, c, x),
			h if h < 240.0 => (0.0, x, c),
			h if h < 300.0 => (x, 0.0, c),
			_ => (c, 0.0, x),
		};

		(
			((r + m) * 255.0) as u8,
			((g + m) * 255.0) as u8,
			((b + m) * 255.0) as u8,
		)
	}

	#[inline]
	fn rgb_from_hue(h: f32) -> (u8, u8, u8) {
		hsv2rgb(360f32 * h, 1.0, 1.0)
	}

	let width = spectra.len();
	let height = spectra[0].len();

	// TODO: everything that touches this vector could use a refactor. Mainly moving from
	// u8 to Rgba struct and using Rgba more broadly elsewhere
	// also the mathy logic here could be moved out of the function or could use some comments
	let mut img = vec![0u8; width * height * 4];
	img.chunks_exact_mut(4)
		.for_each(|chunk| chunk.copy_from_slice(&[0, 0, 0, 255]));

	for (x, spectrum) in spectra.iter_mut().enumerate() {
		let (min, max) = extrema(spectrum.iter());
		let range = CLAMP_FACTOR * (max - min);
		for (y, &value) in spectrum.iter().enumerate() {
			let start = (x + y * width) * 4;
			let normed_hue = ((value - min) / range).clamp(0.0, 1.0);
			let (r, g, b) = rgb_from_hue(normed_hue);

			if normed_hue > CUTOFF {
				img[start..start + 3].copy_from_slice(&[r, g, b]);
			}
		}
	}

	(width, height, img)
}

pub fn show_spectrogram(spectra: (usize, usize, Vec<u8>), ffts_per_second: u32) {
	let (domain, range, img) = spectra;
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
					frame[RGBA * width * y..RGBA * width * (y + 1)].copy_from_slice(
						&img[RGBA * (ps.x_offset + y * domain)
							..RGBA * (ps.x_offset + y * domain + width)],
					);
				}
				ps.inc();
			} else {
				frame.copy_from_slice(&img);
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
