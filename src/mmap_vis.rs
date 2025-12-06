use std::env::args;
use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use crate::parser::{Channels, MmapedRiffPcm, from_mmap};
use cpal::traits::HostTrait as _;
use cpal::traits::{DeviceTrait as _, StreamTrait as _};
use cpal::{BufferSize, SampleRate, StreamConfig};
use memmap2::Mmap;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

pub fn mmap_vis(file_path: String) {
	let file_buf: &'static [u8] = mmap_file(Path::new(&file_path));
	let MmapedRiffPcm {
		samples_per_second,
		channels,
		samples,
	} = from_mmap(file_buf);
	let (tx, rx) = channel();
	dbg!(samples.len() as f64 / channels as u8 as f64 / samples_per_second as f64);
	let _dontdrop =
		spawn_paused_child_audio_thread(rx, samples, samples_per_second, channels as u16);
	run_window(tx, samples, samples_per_second, channels);
}

fn mmap_file(path: &Path) -> &'static [u8] {
	static MMAP: OnceLock<Mmap> = OnceLock::new();
	{
		let file = File::open(path).unwrap();

		// the lifetime of the mmap is not tied to the lifetime of the file descriptor it was
		// created from, so Mmap: 'static
		//
		// SAFETY: this is unsound; we have no reason to think that the file won't be removed
		// while we read it. But we can't do anything about this; libc flock(2) is not strong enough
		// to prevent this, and it's also not cross-platform. So we don't have much of a choice.
		// The memmap2 crate docs guarantee that if we violate this assumption, we will get a
		// SIGBUS (and thus the program will terminate), which means this doesn't violate the
		// "real" memory safety of this program.
		MMAP.set(unsafe { Mmap::map(&file) }.unwrap())
			.expect("the oncelock cannot be initialized yet");
	}
	// 'static :)
	&*(*MMAP.get().expect("the oncelock was just initialized"))
}

enum Action {
	PlayPause,
}

fn spawn_paused_child_audio_thread(
	rx: Receiver<Action>,
	samples: &'static [i16],
	samples_per_second: u32,
	channels: u16,
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

fn run_window(
	tx: Sender<Action>,
	samples: &'static [i16],
	samples_per_second: u32,
	channels: Channels,
) {
	let event_loop = EventLoop::new().unwrap();
	let mut input = WinitInputHelper::new();

	let window = {
		let size = PhysicalSize::new(800, 600);
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

	let mut play_time_from_start = Duration::ZERO;
	let mut started_playing_at = Option::<Instant>::None;

	let _ = event_loop.run(|event, window_hook| {
		if let Event::WindowEvent {
			event: WindowEvent::RedrawRequested,
			..
		} = event
		{
			let frame = pixels.frame_mut();
			let mut time_idx = if let Some(inst) = started_playing_at {
				play_time_from_start + inst.elapsed()
			} else {
				play_time_from_start
			};
			let mut sample_idx = (time_idx.as_secs_f64() * samples_per_second as f64) as usize;

			// TODO: incorrect because of >1 channel
			if sample_idx >= samples.len() {
				started_playing_at = None;
				play_time_from_start = Duration::from_secs(
					samples.len() as u64 / channels as u64 / samples_per_second as u64,
				);
				time_idx = play_time_from_start;
				sample_idx = (time_idx.as_secs_f64() * samples_per_second as f64) as usize;
			}

			// perform fft. idk this is not my job
			frame.fill(255);

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

				match started_playing_at {
					Some(inst) => {
						started_playing_at = None;
						play_time_from_start += inst.elapsed();
					},
					None => {
						started_playing_at = Some(Instant::now());
					},
				}

				return;
			}

			window.request_redraw();
		}
	});
}
