use bytemuck::checked::cast_slice_mut;
use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};
use cpal::{BufferSize, SampleRate, StreamConfig};
use pixels::{Pixels, SurfaceTexture};
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use crate::fft::{MutSlice2D, SPECTRUM_SIZE, STEP_SIZE, WINDOW_SIZE, sliding_spectra};
use crate::graphics::{ColorScheme, draw_vbar, spectrogram_into};
use crate::parser::{Channels, MmapedRiffPcm, Samples, from_mmap, mmap_file};

const WIDTH: usize = 3000;
const MAX_HEIGHT: u32 = WINDOW_SIZE as u32 / 2;

pub fn realtime_vis(file_path: &str, cs: ColorScheme) {
	let mmap: MmapedRiffPcm<'static> = from_mmap(mmap_file(file_path));
	let (tx, rx) = channel();
	let _dontdrop = spawn_audio(rx, mmap);
	run_window(tx, mmap, cs);
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
		buffer_size: BufferSize::Default,
	};

	let mut player_head = 0;
	let mut paused = true;

	let stream = device
		.build_output_stream(
			&config,
			move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
				for event in rx.try_iter() {
					match event {
						Action::PlayPause => {
							paused ^= true;
						},
						Action::Advance => {
							player_head += channels as usize * (samples_per_second / 2) as usize;
						},
						Action::Rewind => {
							player_head = player_head
								.checked_sub(channels as usize * (samples_per_second / 2) as usize)
								.unwrap_or(0);
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

const PRECOMPUTE: usize = 10_000; //Maximum number of FFTs we expect to ever have to precompute
struct FftMaker {
	fft_buf: VecDeque<f32>,
	rbound: usize, // Bounds of what lies in the buffer, in "frames" (sample buffer ignoring channels)
	lbound: usize,
	channels: usize,
	samples: Samples,
}

fn floor_step(x: usize) -> usize {
	x & !(STEP_SIZE - 1)
}

fn ceil_step(x: usize) -> usize {
	(x + STEP_SIZE - 1) & !(STEP_SIZE - 1)
}

impl FftMaker {
	fn new(c: Channels, samples: Samples) -> Self {
		Self {
			fft_buf: VecDeque::with_capacity(PRECOMPUTE * SPECTRUM_SIZE),
			lbound: 0,
			rbound: 0,
			channels: c as usize,
			samples,
		}
	}

	fn r#yield(&mut self, start_frame: usize, out: &mut [f32]) {
		// number of columns == out.len()/SPECTRUM_SIZE
		let start_frame = floor_step(start_frame);
		let end_frame = start_frame + (out.len() / SPECTRUM_SIZE) * STEP_SIZE;

		self.extend_front(end_frame + 3 * WINDOW_SIZE);
		self.extend_back(start_frame.checked_sub(WINDOW_SIZE).unwrap_or(0));

		let (head, tail) = self.fft_buf.as_slices();

		let fft_start = ((start_frame - self.lbound) / STEP_SIZE) * SPECTRUM_SIZE;
		let fft_end = ((end_frame - self.lbound) / STEP_SIZE) * SPECTRUM_SIZE;

		let truelen = fft_end - fft_start;
		// assert!(self.fft_buf.len() >= fft_end);
		// assert!(out.len() == truelen);

		if fft_end <= head.len() {
			out.copy_from_slice(&head[fft_start..fft_end]);
		} else if fft_start < head.len() {
			let front = &head[fft_start..];
			out[..front.len()].copy_from_slice(front);
			out[front.len()..].copy_from_slice(&tail[..truelen - front.len()]);
		} else {
			out.copy_from_slice(&tail[fft_start - head.len()..fft_end - head.len()]);
		}

		self.drop_back(start_frame.checked_sub(WINDOW_SIZE).unwrap_or(0));
	}

	pub fn extend_front(&mut self, right_frame: usize) {
		let true_right = ceil_step(right_frame);
		if true_right <= self.rbound {
			return;
		}

		let raw_l = self.rbound * self.channels;
		let raw_r = (true_right + WINDOW_SIZE) * self.channels;

		if raw_l >= self.samples.len() {
			return;
		}

		let mono: Vec<f32> = self.samples[raw_l..raw_r.min(self.samples.len())]
			.chunks_exact(self.channels)
			.map(|x| x[0] as f32 / i16::MAX as f32)
			.collect();

		let new_ffts = sliding_spectra(&mono);
		self.fft_buf.extend(new_ffts.data);

		self.rbound = true_right;
	}

	pub fn extend_back(&mut self, start_frame: usize) {
		let true_start = floor_step(start_frame);
		if true_start >= self.lbound {
			return;
		}

		let raw_l = true_start * self.channels;
		let raw_r = (self.lbound + WINDOW_SIZE) * self.channels;

		let mono: Vec<f32> = self.samples[raw_l..raw_r.min(self.samples.len())]
			.chunks_exact(self.channels)
			.map(|x| x[0] as f32 / i16::MAX as f32)
			.collect();

		let new_ffts = sliding_spectra(&mono);
		for &datum in new_ffts.data.iter().rev() {
			self.fft_buf.push_front(datum);
		}

		self.lbound = true_start;
	}

	pub fn drop_back(&mut self, new_left_frame: usize) {
		let new_left = floor_step(new_left_frame);
		if new_left <= self.lbound {
			return;
		}
		let drop_cols = (new_left - self.lbound) / STEP_SIZE;
		self.fft_buf.drain(..drop_cols * SPECTRUM_SIZE);
		self.lbound += drop_cols * STEP_SIZE;
	}
}

#[derive(Debug)]
struct SongTime {
	start_timestamp: Duration,
	started_playing: Option<Instant>,
	song_length: Duration,
}

impl SongTime {
	fn new(song_length: Duration) -> Self {
		Self {
			start_timestamp: Duration::ZERO,
			started_playing: None,
			song_length,
		}
	}

	fn sample_idx(&self, samples_per_second: usize) -> usize {
		return (self.dur().as_secs_f64() * samples_per_second as f64) as usize;
	}

	fn check_end(&mut self) {
		if self.dur() >= self.song_length {
			self.started_playing = None;
			self.start_timestamp = self.song_length;
		}
	}

	fn dur(&self) -> Duration {
		self.start_timestamp + self.started_playing.map_or(Duration::ZERO, |t| t.elapsed())
	}

	fn handle(&mut self, act: Action) {
		match act {
			Action::PlayPause => {
				if let Some(inst) = self.started_playing {
					self.started_playing = None;
					self.start_timestamp += inst.elapsed();
				} else {
					self.started_playing = Some(Instant::now());
				}
			},
			Action::Advance => {
				self.start_timestamp += Duration::from_secs_f32(0.5);
				self.check_end()
			},
			Action::Rewind => {
				if let Some(inst) = self.started_playing {
					self.start_timestamp += inst.elapsed();
					self.start_timestamp = self
						.start_timestamp
						.checked_sub(Duration::from_secs_f32(0.5))
						.unwrap_or_default();
					self.started_playing = Some(Instant::now());
				} else {
					self.start_timestamp = self
						.start_timestamp
						.checked_sub(Duration::from_secs_f32(0.5))
						.unwrap_or_default();
				}
			},
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
	cs: ColorScheme,
) {
	let event_loop = EventLoop::new().unwrap();
	let mut input = WinitInputHelper::new();
	let mut display_width = WIDTH as u32;

	let window = {
		let size = PhysicalSize::new(display_width, WINDOW_SIZE as u32 / 2);
		WindowBuilder::new()
			.with_title("")
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

	let mut pixel_scale = 3;
	let mut visual_sensitivity: f32 = 1.0;

	let mut read_buf: Box<[f32]> = vec![0.0f32; PRECOMPUTE * SPECTRUM_SIZE].into();
	let mut maker: FftMaker = FftMaker::new(channels, samples);
	let mut song: SongTime = SongTime::new(Duration::from_millis(
		(1000 * samples.len() as u64 / channels as u64 / samples_per_second as u64) as u64 - 101,
	));
	let _ = event_loop.run(|event, window_hook| {
		if let Event::WindowEvent {
			event: WindowEvent::RedrawRequested,
			..
		} = event
		{
			song.check_end();
			let frame_width = display_width / pixel_scale;
			let mut frame = MutSlice2D {
				data: cast_slice_mut(pixels.frame_mut()),
				width: frame_width as usize,
			};

			let n_frames = samples.len() / channels as usize;

			let mut center = song.sample_idx(samples_per_second as usize);
			center = center.min(n_frames);

			let half = (frame_width as usize / 2) * STEP_SIZE;
			let span = (frame_width as usize) * STEP_SIZE;

			let mut left = center.saturating_sub(half);
			let mut right = left + span;

			let right_max = n_frames.saturating_sub(WINDOW_SIZE);
			if right > right_max {
				right = right_max;
				left = right.saturating_sub(span);
			}

			let demand = SPECTRUM_SIZE * frame_width as usize;
			let prerender = MutSlice2D {
				data: &mut read_buf[..demand],
				width: SPECTRUM_SIZE,
			};

			maker.r#yield(left, prerender.data);
			spectrogram_into(prerender.into(), visual_sensitivity, frame.reborrow(), cs);

			let bar_location = (center - left) / STEP_SIZE;
			draw_vbar(bar_location, frame.reborrow());

			if pixels.render().is_err() {
				window_hook.exit();
				return;
			}
		}

		if input.update(&event) {
			if input.key_pressed(KeyCode::KeyQ) || input.close_requested() {
				window_hook.exit();
				return;
			}

			if input.key_pressed(KeyCode::Space) {
				tx.send(Action::PlayPause).unwrap();
				song.handle(Action::PlayPause);
			}

			if input.key_pressed_os(KeyCode::ArrowRight) {
				tx.send(Action::Advance).unwrap();
				song.handle(Action::Advance);
			}
			if input.key_pressed_os(KeyCode::ArrowLeft) {
				tx.send(Action::Rewind).unwrap();
				song.handle(Action::Rewind);
			}

			if input.key_pressed_os(KeyCode::Equal) || input.scroll_diff().1 > 0.0 {
				pixel_scale += 1;
				resize_pixels(&mut pixels, window.inner_size(), pixel_scale);
			}

			if input.key_pressed_os(KeyCode::Minus) || input.scroll_diff().1 < 0.0 {
				if pixel_scale > 1 {
					pixel_scale -= 1;
					resize_pixels(&mut pixels, window.inner_size(), pixel_scale);
				}
			}

			if input.key_pressed_os(KeyCode::ArrowUp) {
				visual_sensitivity += 0.1;
			}
			if input.key_pressed_os(KeyCode::ArrowDown) {
				visual_sensitivity = (visual_sensitivity - 0.1).clamp(-1.0, 5.0);
			}

			window.request_redraw();
		}
		if let Event::WindowEvent {
			event: WindowEvent::Resized(size),
			..
		} = event
		{
			resize_pixels(&mut pixels, size, pixel_scale);
			display_width = size.width;
		}
	});
}
fn resize_pixels(pixels: &mut Pixels, size: PhysicalSize<u32>, pixel_scale: u32) {
	pixels
		.resize_surface(size.width, size.height.clamp(1, MAX_HEIGHT * pixel_scale))
		.unwrap();
	pixels
		.resize_buffer(
			size.width / pixel_scale,
			(size.height / pixel_scale).clamp(1, MAX_HEIGHT),
		)
		.unwrap();
}
