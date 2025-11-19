#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::Fix;

const Y_PIXEL_SCALE: u32 = 4;
const X_PIXEL_SCALE: u32 = 2;

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

pub fn draw_fft(ffts: &[Vec<Fix>]) {
	let event_loop = EventLoop::new().unwrap();
	let mut input = WinitInputHelper::new();
	let width = ffts.len() as u32;
	let height = ffts[0].len() as u32;
	let window = {
		let size = PhysicalSize::new(
			width as f64 * X_PIXEL_SCALE as f64,
			height as f64 * Y_PIXEL_SCALE as f64,
		);
		WindowBuilder::new()
			.with_title("Hello Pixels")
			.with_inner_size(size)
			.with_min_inner_size(size)
			.build(&event_loop)
			.unwrap()
	};

	let mut pixels = {
		let window_size = window.inner_size();
		let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
		Pixels::new(width, height, surface_texture).unwrap()
	};

	let res = event_loop.run(|event, elwt| {
		// Draw the current frame
		if let Event::WindowEvent {
			event: WindowEvent::RedrawRequested,
			..
		} = event
		{
			let frame = pixels.frame_mut();
			frame.chunks_exact_mut(4).for_each(|chunk| {
				chunk[0] = 0;
				chunk[1] = 0;
				chunk[2] = 0;
				chunk[3] = 255;
			});

			let spectrum = ffts
				.chunks_exact(2)
				.map(|fft| {
					fft[0]
						.iter()
						.zip(fft[1].iter())
						.map(|(&value, &value2)| (value).powi(2) + value2.powi(2))
						.collect::<Vec<_>>()
				})
				.collect::<Vec<_>>();
			fn activation(x: f32) -> f32 {
				x.sqrt()
			}

			for (x, freqs) in spectrum.iter().enumerate() {
				let mi = freqs.iter().fold(f32::MAX, |acc, &x| acc.min(x));
				let ma = freqs.iter().fold(f32::MIN, |acc, &x| acc.max(x));
				for (y, &value) in freqs.iter().enumerate() {
					let start = (x as usize + y as usize * width as usize) * 4;
					let norm = activation(value / (ma - mi));
					let (r, g, b) = hsv_to_rgb(norm * 360f32, 1.0, 1.0);

					if norm > 0.1 {
						frame[start] = r as u8;
						frame[start + 1] = g as u8;
						frame[start + 2] = b as u8;
						frame[start + 3] = 255;
					}
				}
			}

			if let Err(err) = pixels.render() {
				elwt.exit();
				return;
			}
		}

		if input.update(&event) {
			if input.key_pressed(KeyCode::Escape) || input.close_requested() {
				elwt.exit();
				return;
			}

			// Resize the window
			if let Some(size) = input.window_resized() {
				if let Err(err) = pixels.resize_surface(size.width, size.height) {
					elwt.exit();
					return;
				}
			}

			// update internal state here
			window.request_redraw();
		}
	});
}
