#![forbid(unsafe_code)]

use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::{Cplx, Float};

const PIXEL_SCALE: u32 = 2;
const MAX_WIDTH: usize = 1500;
const BYTES_PER_PIXEL: usize = 4;

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

fn rgb_from_hue(h: f32) -> (u8, u8, u8) {
	hsv_to_rgb(360f32 * h, 1.0, 1.0)
}

pub(crate) fn draw_spectra(spectra: &Vec<Vec<Float>>, ffts_per_second: u32) {
	let width = spectra.len();
	let height = spectra.first().unwrap().len();

	let mut img = vec![0u8; width * height * 4];
	img.chunks_exact_mut(4).for_each(|chunk| {
		chunk[0] = 0;
		chunk[1] = 0;
		chunk[2] = 0;
		chunk[3] = 255;
	});

	fn activation(x: f32) -> f32 {
		5f32 * x
	}

	for (x, spectrum) in spectra.iter().enumerate() {
		let mi = spectrum.iter().fold(f32::MAX, |acc, &x| acc.min(x));
		let ma = spectrum.iter().fold(f32::MIN, |acc, &x| acc.max(x));
		for (y, &value) in spectrum.iter().enumerate() {
			let start = (x as usize + y as usize * width as usize) * 4;
			let normed_hue = activation(value / (ma - mi));
			let (r, g, b) = rgb_from_hue(normed_hue);

			if normed_hue > 0.05 {
				img[start] = r as u8;
				img[start + 1] = g as u8;
				img[start + 2] = b as u8;
				img[start + 3] = 255;
			}
		}
	}

	display_static(width, height, img, ffts_per_second);
}

fn display_static(width: usize, height: usize, image: Vec<u8>, ffts_per_second: u32) {
	let event_loop = EventLoop::new().unwrap();
	let mut input = WinitInputHelper::new();
	let scrollable: bool = width > MAX_WIDTH;
	dbg!(scrollable);
	let pixel_width = if scrollable { MAX_WIDTH as f64 } else { width as f64 };
	dbg!(pixel_width);
	let mut x_offset: usize = 0;
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
			.build(&event_loop)
			.unwrap()
	};

	let mut pixels = {
		let window_size = window.inner_size();
		let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
		Pixels::new(pixel_width as u32, height as u32, surface_texture).unwrap()
	};

	let fft_period = 1f64 / ffts_per_second as f64;
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
			if let Err(err) = pixels.render() {
				elwt.exit();
				return;
			}
		}

		if input.update(&event) {
			if scrollable {
				let x_scroll = -input.scroll_diff().0;
				if x_scroll > 0.0 {
					let new_pos = x_offset + x_scroll as usize;
					if new_pos < width - pixel_width as usize {
						x_offset = new_pos;
					}
				} else if x_scroll < 0.0 {
					let new_pos = x_offset as f64 - (-x_scroll as f64);
					if new_pos > 0.0 {
						x_offset = new_pos as usize;
					}
				}
			}

			if input.key_pressed(KeyCode::KeyQ) || input.close_requested() {
				elwt.exit();
				return;
			}

			if input.key_pressed(KeyCode::Space) {
				play = !play;
			}

			if let Some(size) = input.window_resized() {
				if let Err(err) = pixels.resize_surface(size.width, size.height) {
					elwt.exit();
					return;
				}
			}

			window.set_title(&format!(
				"Viewing {}:{} of {}, corresponds to {}s to {}s",
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
