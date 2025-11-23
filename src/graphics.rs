#![forbid(unsafe_code)]

use pixels::{Pixels, SurfaceTexture};
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::Float;

const PIXEL_SCALE: usize = 2;
const MAX_WIDTH: usize = 1500;
const RGBA: usize = 4; //Magic number for bytes/color
const INERTIA_RATIO: f32 = 5f32 / 6f32;
const CUTOFF: f32 = 0.05;
const CLAMP_FACTOR: f32 = 1.0;

struct PlayState {
	pub x_offset: usize,
	pub scroll_v: f32,
	pub playing: Option<(Instant, usize)>,
	pub ffts_per_second: u32,
}

impl PlayState {
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
		let scroll_out = apply_inertia(self.scroll_v, scroll_in) as isize;
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
		self.scroll_v = apply_inertia(self.scroll_v, scroll_out as f32);
	}
}

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

	return (
		((r + m) * 255.0) as u8,
		((g + m) * 255.0) as u8,
		((b + m) * 255.0) as u8,
	);
}

#[inline]
fn rgb_from_hue(h: f32) -> (u8, u8, u8) {
	hsv2rgb(360f32 * h, 1.0, 1.0)
}

#[inline]
///Did this because f32 doesn't implement Ord, so can't just use .max()/.min()
/// TODO: figure out if theres an easier way
fn extrema<'a>(v: impl Iterator<Item = &'a f32>) -> (f32, f32) {
	v.fold((f32::MAX, f32::MIN), |(curr_min, curr_max), &x| {
		(curr_min.min(x), curr_max.max(x))
	})
}

pub(crate) fn generate_spectrogram(spectra: &mut Vec<Vec<Float>>, ffts_per_second: u32) {
	let width = spectra.len();
	let height = spectra[0].len();

	let mut img = vec![0u8; width * height * 4];
	img.chunks_exact_mut(4)
		.for_each(|chunk| chunk.copy_from_slice(&[0, 0, 0, 255]));

	for (x, spectrum) in spectra.into_iter().enumerate() {
		fn gain(x: &f32) -> f32 {
			*x
		}
		spectrum.iter_mut().for_each(|x| *x = gain(x));
		let (mi, ma) = extrema(spectrum.iter());
		let range = CLAMP_FACTOR * (ma - mi);
		for (y, &value) in spectrum.iter().enumerate() {
			let start = (x as usize + y as usize * width as usize) * 4;
			let normed_hue = ((value - mi) / range).clamp(0.0, 1.0);
			let (r, g, b) = rgb_from_hue(normed_hue);

			if normed_hue > CUTOFF {
				img[start..start + 3].copy_from_slice(&[r, g, b]);
			}
		}
	}

	show_spectrogram(width, height, img, ffts_per_second);
}

fn show_spectrogram(domain: usize, range: usize, image: Vec<u8>, ffts_per_second: u32) {
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
			if let Some(sc) = play.as_mut() {
				draw_subview(frame, &image, range, width, domain, sc.x_offset);
				sc.inc();
			} else {
				frame.copy_from_slice(&image);
			}

			if let Err(_) = pixels.render() {
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

				window.set_title(&format!(
					"Viewing {}:{} of {}, corresponds to {:.3}s to {:.3}s",
					ps.x_offset,
					ps.x_offset + width,
					domain,
					(ps.x_offset as f64 / ffts_per_second as f64),
					(ps.x_offset + width) as f64 / ffts_per_second as f64
				));
			}

			window.request_redraw();
		}
	});
}
#[inline]
fn apply_inertia(inertia: f32, delta: f32) -> f32 {
	(INERTIA_RATIO) * inertia + (1f32 - INERTIA_RATIO) * delta
}

#[inline]
fn draw_subview(
	frame: &mut [u8],
	image: &[u8],
	range: usize,
	width: usize,
	domain: usize,
	x_offset: usize,
) {
	for y in 0..range {
		frame[RGBA * width * y..RGBA * width * (y + 1)].copy_from_slice(
			&image[RGBA * (x_offset + y * domain)..RGBA * (x_offset + y * domain + width)],
		);
	}
}
