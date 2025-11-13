#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 321;
const HEIGHT: u32 = 240;
const PIXEL_SCALE: u32 = 8;

/// Representation of the application state. In this example, a box will bounce around the screen.
struct World {
	box_x: i16,
	box_y: i16,
	velocity_x: i16,
	velocity_y: i16,
}

pub fn smain() {
	let event_loop = EventLoop::new().unwrap();
	let mut input = WinitInputHelper::new();
	let window = {
		let size = PhysicalSize::new(
			WIDTH as f64 * PIXEL_SCALE as f64,
			HEIGHT as f64 * PIXEL_SCALE as f64,
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
		Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap()
	};

	let res = event_loop.run(|event, elwt| {
		// Draw the current frame
		if let Event::WindowEvent {
			event: WindowEvent::RedrawRequested,
			..
		} = event
		{
			// world.draw(pixels.frame_mut());
			pixels
				.frame_mut()
				.chunks_mut(8)
				.for_each(|chunk| chunk[0..4].fill(255));
			//DRAW HERE
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
