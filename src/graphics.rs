#![forbid(unsafe_code)]

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::Float;

const PIXEL_SCALE: u32 = 2;
const MAX_WIDTH: usize = 1500;
const BYTES_PER_PIXEL: usize = 4;
const INERTIA_RATIO: f32 = 5f32 / 6f32;
const CUTOFF: f32 = 0.05;
const CLAMP_FACTOR: f32 = 1.0;

#[inline]
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
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
	hsv_to_rgb(360f32 * h, 1.0, 1.0)
}

#[inline]
fn apply_inertia(inertia: f32, delta: f32) -> f32 {
	(INERTIA_RATIO) * inertia + (1f32 - INERTIA_RATIO) * delta
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
			// x.powi(2)
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

fn show_spectrogram(width: usize, height: usize, image: Vec<u8>, ffts_per_second: u32) {
	let event_loop = EventLoop::new().unwrap();
	let mut input = WinitInputHelper::new();
	let scrollable: bool = width > MAX_WIDTH;
	dbg!(scrollable);
	let pixel_width = if scrollable { MAX_WIDTH as f64 } else { width as f64 };
	dbg!(pixel_width);
	let mut x_offset: usize = 0;
	let mut scroll_v: f32 = 0.0;
	let mut play: bool = false;

	let window = {
		let size = PhysicalSize::new(
			pixel_width * PIXEL_SCALE as f64,
			height as f64 * PIXEL_SCALE as f64,
		);
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
		Pixels::new(pixel_width as u32, height as u32, surface_texture).unwrap()
	};

	let _ = event_loop.run(|event, elwt| {
		if let Event::WindowEvent {
			event: WindowEvent::RedrawRequested,
			..
		} = event
		{
			let frame = pixels.frame_mut();
			if !scrollable {
				frame.copy_from_slice(&image);
			} else {
				for y in 0..height {
					frame[y * pixel_width as usize * BYTES_PER_PIXEL
						..(y + 1) * pixel_width as usize * BYTES_PER_PIXEL]
						.copy_from_slice(
							&image[BYTES_PER_PIXEL * (x_offset + y * width as usize)
								..BYTES_PER_PIXEL
									* (x_offset + y * width as usize + pixel_width as usize)],
						);
				}
			}

			if play {
				x_offset += 3;
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

			if !scrollable {
				return;
			}

			let scroll = apply_inertia(scroll_v, input.scroll_diff().1);
			match scroll {
				(0.0..) => {
					let new_pos = x_offset as isize + scroll as isize;
					if new_pos < width as isize - pixel_width as isize {
						x_offset = new_pos as usize;
					}
				},
				(..0.0) => {
					let new_pos = x_offset as isize + scroll as isize;
					if new_pos > 0 {
						x_offset = new_pos as usize;
					}
				},
				_ => {},
			}
			scroll_v = apply_inertia(scroll_v, scroll);

			if input.key_pressed(KeyCode::Space) {
				play = !play;
			}

			let fft_period = 1f64 / ffts_per_second as f64;
			window.set_title(&format!(
				"Viewing {}:{} of {}, corresponds to {:.3}s to {:.3}s",
				x_offset,
				x_offset + pixel_width as usize,
				width,
				(x_offset as f64 * fft_period),
				(x_offset + pixel_width as usize) as f64 * fft_period
			));
			window.request_redraw();
		}
	});
}
