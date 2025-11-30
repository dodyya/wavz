use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;

use bytemuck::cast_slice;

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::BoxSlice2D;
use crate::fft::RESOLUTION;
use crate::fft::fft_spectrum;
use crate::graphics::render_spectrum;
use crate::rgba::Rgba;

const PIXEL_SCALE: usize = 2;
const MAX_WIDTH: usize = 1500; // Maximum screen width, determines playability
const RGBA: usize = 4;

#[derive(Debug)]
enum FftEvent {
	PixelsReady { pix: Arc<[Rgba]> },
}
pub fn mic_into_pixels() {
	const STEP_SIZE: usize = 1 << 9;
	use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};
	use ringbuf::HeapRb;
	use ringbuf::storage::Heap;
	use ringbuf::traits::{Consumer as _, Producer as _, Split as _};
	use std::sync::mpsc::channel;
	use std::time::Duration;
	use winit::event_loop::EventLoopBuilder;
	let host = cpal::default_host();

	let device = host.default_input_device().unwrap();

	let config = device.default_input_config().unwrap();

	let err_fn = move |err| {
		eprintln!("an error occurred on stream: {err}");
	};

	let (mut mic_prod, mut mic_cons) = HeapRb::<f32>::new(RESOLUTION * 2).split();

	let stream = match config.sample_format() {
		cpal::SampleFormat::F32 => device
			.build_input_stream(
				&config.into(),
				move |data: &[f32], _: &_| {
					mic_prod.push_slice(data);
				},
				err_fn,
				None,
			)
			.unwrap(),
		sample_format => {
			panic!("Unsupported sample format '{sample_format}'")
		},
	};

	let event_loop = EventLoopBuilder::<FftEvent>::with_user_event()
		.build()
		.unwrap();

	let send_proxy = event_loop.create_proxy();

	thread::spawn(move || {
		let mut mic_buf = Vec::<f32>::with_capacity(RESOLUTION * 20);
		let mut incoming = vec![0.0f32; RESOLUTION];
		let mut idx: usize = 0;

		loop {
			let n = mic_cons.pop_slice(&mut incoming);

			if n == 0 {
				thread::sleep(Duration::from_micros(200));
				continue;
			}

			mic_buf.extend_from_slice(&incoming[..n]);

			while idx + RESOLUTION < mic_buf.len() {
				send_proxy
					.send_event(FftEvent::PixelsReady {
						pix: Arc::from(
							render_spectrum(&fft_spectrum(&mic_buf[idx..idx + RESOLUTION]))
								.into_boxed_slice(),
						),
					})
					.expect("Failed to send event");
				idx += STEP_SIZE;
			}

			if idx > RESOLUTION * 16 {
				mic_buf.copy_within(idx.., 0);
				mic_buf.truncate(mic_buf.len() - idx);
				idx = 0;
			}
		}
	})
	.join()
	.unwrap();

	let _ = stream.play();
	show_mic(event_loop);
	drop(stream);
}

pub fn show_mic(event_loop: EventLoop<FftEvent>) {
	use crate::fft::RESOLUTION;

	let mut input = WinitInputHelper::new();
	let height = RESOLUTION / 2;
	let width = MAX_WIDTH;

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
			const STEP: usize = 5;

			for y in 0..height {
				frame.copy_within(
					y * width * RGBA + STEP * RGBA..(y + 1) * width * RGBA,
					y * width * RGBA,
				);
			}

			if pixels.render().is_err() {
				elwt.exit();
				return;
			}
		}

		if let Event::UserEvent(FftEvent::PixelsReady { pix }) = &event {
			println!("Event received.");
			println!("{}", pix.len());
		}

		if input.update(&event) {
			if input.key_pressed(KeyCode::KeyQ) || input.close_requested() {
				elwt.exit();
				return;
			}

			// if let Some(ps) = play.as_mut() {
			// 	ps.handle_scroll(input.scroll_diff().1, domain as isize, width as isize);

			// 	window.set_title(&format!("{domain} samples generated. {ps:?}"));
			// }

			window.request_redraw();
		}
	});
}
