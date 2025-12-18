use bytemuck::checked::cast_slice_mut;
use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};
use cpal::{BufferSize, SampleRate, StreamConfig};
use memmap2::Mmap;
use pixels::{Pixels, SurfaceTexture};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer as _, Observer as _, Producer as _};
use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::{MutSlice2D, SPECTRUM_SIZE, STEP_SIZE, Slice2D, WINDOW_SIZE, sliding_spectra};
use crate::parser::{Channels, MmapedRiffPcm, Samples, from_mmap, mmap_file};

const WIDTH: usize = 2000;
const MAX_HEIGHT: u32 = WINDOW_SIZE as u32 / 2;

pub fn realtime_vis(file_path: &str) {
	// let file_buf: &'static [u8] = ;
	let mmap: MmapedRiffPcm<'static> = from_mmap(mmap_file(file_path));
	let (tx, rx) = channel();
	let _dontdrop = spawn_audio(rx, mmap);
	run_window(tx, mmap);
}

enum Action {
	PlayPause,
	Advance,
	Rewind,
}

fn spawn_audio(
	rx: Receiver<Action>,
	MmapedRiffPcm {
		samples,
		samples_per_second,
		channels,
	}: MmapedRiffPcm<'static>,
) -> cpal::Stream {
	let host = cpal::default_host();

	#[cfg(not(target_os = "linux"))]
	let device = host.default_output_device().unwrap();
	#[cfg(target_os = "linux")]
	let device = host
		.output_devices()
		.unwrap()
		.find(|dev| dev.name().as_deref() == Ok("pipewire"))
		.unwrap();

	let config = StreamConfig {
		channels: channels as u16,
		sample_rate: SampleRate(samples_per_second),
		// TODO: lower this value so that the audio thread is more responsive
		buffer_size: BufferSize::Default,
	};

	let mut player_head = 0;
	let mut paused = true;

	let stream = device
		.build_output_stream(
			&config,
			move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
				for event in rx.try_iter() {
					// flip play/pause, seek player_head, restart track, ...
					match event {
						Action::PlayPause => {
							paused ^= true;
						},
						Action::Advance => {
							player_head += 10000;
						},
						Action::Rewind => {
							player_head -= 10000;
						},
					}
				}

				if paused {
					data.fill(0);
					return;
				}

				if let Some(slice) = samples.get(player_head..player_head + data.len()) {
					data.copy_from_slice(slice);
					player_head += data.len();
				} else {
					let consumed = samples.len() - player_head;
					data[..consumed].copy_from_slice(&samples[player_head..]);
					data[consumed..].fill(0);
					player_head += consumed;
					paused = true;
				}
			},
			move |e| panic!("audio thread encountered an error: {e}"),
			None,
		)
		.unwrap();

	stream.play().unwrap();
	stream
}

const PRECOMPUTE: usize = 10000; //Maximum number of FFTs we expect to ever have to precompute
struct FftProcessor {
	fft_buf: HeapRb<f32>,
	discard_buf: Box<[f32]>,
	fft_head: usize,
}

impl FftProcessor {
	pub fn new() -> Self {
		let fft_buf = HeapRb::<f32>::new(PRECOMPUTE * SPECTRUM_SIZE);
		let discard_buf = vec![0.0; PRECOMPUTE * SPECTRUM_SIZE].into_boxed_slice();
		let fft_head = 0;

		Self { fft_buf, discard_buf, fft_head }
	}

	pub fn yield_ffts(&self, out: &mut [f32]) {
		self.fft_buf.peek_slice(out);
	}

	pub fn process_chunk(&mut self, samples: Samples, delta: usize, channels: usize) {
		let new_ffts = sliding_spectra(
			&samples[self.fft_head
				..self.fft_head + ((delta - 1) * STEP_SIZE + WINDOW_SIZE) * channels as usize]
				.chunks_exact(channels as usize)
				.map(|x| x[0] as f32 / i16::MAX as f32)
				.collect::<Vec<_>>(),
		);

		self.fft_buf.push_slice(&new_ffts.data);
		self.fft_head += delta * STEP_SIZE * channels as usize;
	}

	pub fn drain_except(&mut self, keep_cols: usize) {
		if self.fft_buf.occupied_len() > SPECTRUM_SIZE * keep_cols {
			let _ = self.fft_buf.pop_slice(
				&mut self.discard_buf[..self.fft_buf.occupied_len() - SPECTRUM_SIZE * keep_cols],
			);
		}
	}
}

struct Playback {
	timestamp: Duration,
	last_play: Option<Instant>,
	song_length: Duration,
}

impl Playback {
	fn new(song_length: Duration) -> Self {
		Self {
			timestamp: Duration::ZERO,
			last_play: None,
			song_length,
		}
	}

	fn sample_idx(&self, samples_per_second: usize) -> usize {
		return (self.dur().as_secs_f64() * samples_per_second as f64) as usize;
	}

	fn check_end(&mut self) {
		if self.dur() >= self.song_length {
			self.last_play = None;
			self.timestamp = self.song_length;
		}
	}

	fn dur(&self) -> Duration {
		self.timestamp + self.last_play.map_or(Duration::ZERO, |t| t.elapsed())
	}

	fn handle(&mut self, act: Action) {
		match act {
			Action::PlayPause => {
				if let Some(inst) = self.last_play {
					self.last_play = None;
					self.timestamp += inst.elapsed();
				} else {
					self.last_play = Some(Instant::now());
				}
			},
			Action::Advance => todo!(),
			Action::Rewind => todo!(),
		}
	}
}

fn run_window(
	tx: Sender<Action>,
	MmapedRiffPcm {
		samples,
		samples_per_second,
		channels,
	}: MmapedRiffPcm<'static>,
) {
	let event_loop = EventLoop::new().unwrap();
	let mut input = WinitInputHelper::new();
	let mut display_width = WIDTH as u32;

	let window = {
		let size = PhysicalSize::new(display_width, WINDOW_SIZE as u32 / 2);
		WindowBuilder::new()
			.with_title("wavez")
			.with_inner_size(size)
			.with_resizable(true)
			.build(&event_loop)
			.unwrap()
	};

	let mut pixels = {
		let window_size = window.inner_size();
		let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
		Pixels::new(window_size.width, window_size.height, surface_texture).unwrap()
	};

	let mut pixel_scale = 1;
	let mut visual_sensitivity: f32 = 0.05;

	let mut read_buf: Box<[f32]> = vec![0.0f32; PRECOMPUTE * SPECTRUM_SIZE].into();
	let mut prev_fft_idx = 0usize.wrapping_sub(1); //-1 :)
	let mut processor: FftProcessor = FftProcessor::new();
	let mut playback: Playback = Playback::new(Duration::from_secs(
		samples.len() as u64 / channels as u64 / samples_per_second as u64,
	));
	let _ = event_loop.run(|event, window_hook| {
		if let Event::WindowEvent {
			event: WindowEvent::RedrawRequested,
			..
		} = event
		{
			let frame = pixels.frame_mut();
			let frame_width = display_width / pixel_scale;
			let sample_idx = playback.sample_idx(samples_per_second as usize);
			playback.check_end();

			// Calculate how many pixels just went off screen.
			let curr_fft_index = sample_idx / STEP_SIZE;
			let delta = curr_fft_index.wrapping_sub(prev_fft_idx);
			if delta == 0 {
				return;
			}

			prev_fft_idx = curr_fft_index;
			processor.process_chunk(&samples, delta, channels as usize);
			let demand = SPECTRUM_SIZE * frame_width as usize;
			processor.yield_ffts(&mut read_buf[..demand]);

			crate::graphics::gen_spectrogram_into(
				Slice2D {
					data: &read_buf[..demand],
					width: SPECTRUM_SIZE,
				},
				visual_sensitivity,
				MutSlice2D {
					data: cast_slice_mut(frame),
					width: frame_width as usize,
				},
			);

			processor.drain_except(frame_width as usize);

			if pixels.render().is_err() {
				window_hook.exit();
				return;
			}
		}

		let mut resize_pixels = |size: PhysicalSize<u32>, pixel_scale: u32| {
			pixels
				.resize_surface(size.width, size.height.clamp(1, MAX_HEIGHT))
				.unwrap();
			pixels
				.resize_buffer(
					size.width / pixel_scale,
					(size.height / pixel_scale).clamp(1, MAX_HEIGHT / pixel_scale),
				)
				.unwrap();
		};

		if input.update(&event) {
			if input.key_pressed(KeyCode::KeyQ) || input.close_requested() {
				window_hook.exit();
				return;
			}

			if input.key_pressed(KeyCode::Space) {
				tx.send(Action::PlayPause).unwrap();
				playback.handle(Action::PlayPause);
			}

			if input.key_pressed(KeyCode::ArrowRight) {
				tx.send(Action::Advance).unwrap();
				playback.handle(Action::Advance);
			}
			if input.key_pressed(KeyCode::ArrowLeft) {
				tx.send(Action::Rewind).unwrap();
				playback.handle(Action::Rewind);
			}

			if input.key_pressed_os(KeyCode::Equal) || input.scroll_diff().1 > 0.0 {
				pixel_scale += 1;
				resize_pixels(window.inner_size(), pixel_scale);
			}

			if input.key_pressed_os(KeyCode::Minus) || input.scroll_diff().1 < 0.0 {
				if pixel_scale > 1 {
					pixel_scale -= 1;
					resize_pixels(window.inner_size(), pixel_scale);
				}
			}

			if input.key_pressed_os(KeyCode::ArrowUp) {
				visual_sensitivity /= 1.1;
			}
			if input.key_pressed_os(KeyCode::ArrowDown) {
				visual_sensitivity *= 1.1;
			}

			window.request_redraw();
		}
		if let Event::WindowEvent {
			event: WindowEvent::Resized(size),
			..
		} = event
		{
			resize_pixels(size, pixel_scale);
			display_width = size.width;
		}
	});
}
