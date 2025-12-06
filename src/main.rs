use std::env::args;
use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use cpal::traits::HostTrait as _;
use cpal::traits::{DeviceTrait as _, StreamTrait as _};
use cpal::{BufferSize, SampleRate, StreamConfig};
use memmap2::Mmap;
use pixels::{Pixels, SurfaceTexture};
use wavez::parser::{Channels, MmapedRiffPcm, from_mmap};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

mod demos {
	use std::fs::File;
	use std::io::{Read, Seek};
	#[cfg(unix)]
	use std::os::fd::IntoRawFd;
	#[cfg(windows)]
	use std::os::windows::io::IntoRawHandle;
	use std::path::Path;
	use std::sync::atomic::{AtomicBool, Ordering};
	use std::sync::{LazyLock, OnceLock};
	use std::thread;
	use std::time::Duration;

	use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
	use cpal::{BufferSize, SampleFormat, SampleRate, StreamConfig};
	use memmap2::Mmap;
	use wavez::fft::fft_spectrum;
	use wavez::graphics::gen_spectrogram;
	use wavez::parser::{MmapedRiffPcm, RiffWavePcm, from_mmap};
	use wavez::static_vis::{show_spectrogram, sliding_spectra};

	#[allow(unused)]
	pub fn wav_visualizer(data: impl Read + Seek) {
		let RiffWavePcm {
			samples,
			samples_per_second: smps,
		} = RiffWavePcm::parse(data).unwrap();

		let step_size = 1 << 8;
		let spectra = gen_spectrogram(sliding_spectra(
			samples
				.into_iter()
				.map(|x| x as f32 / i16::MAX as f32)
				.collect(),
			step_size,
		));
		show_spectrogram(spectra, smps / step_size as u32);
	}
	#[allow(unused)]
	pub fn mic_input() {
		use wavez::fft::WINDOW_SIZE;
		fn ascii_display(spectrum: &[f32]) {
			let mut buf = String::new();
			for x in spectrum.chunks_exact(14) {
				let max_amp = x.iter().fold(0.0f32, |acc, &x| acc.max(x));
				buf.push_str(match max_amp {
					(..0.0001) => " ",
					(..0.0002) => ".",
					(..0.0004) => "+",
					(..0.0006) => "*",
					(..0.0010) => "#",
					(..0.0020) => "$",
					_ => "@",
				});
			}
			println!("{buf}");
		}

		let host = cpal::default_host();
		let device = host.default_input_device().unwrap();
		let config = device.default_input_config().unwrap();
		println!("{:?}", config);
		let err_fn = move |err| {
			eprintln!("an error occurred on stream: {err}");
		};

		let mut buf = Vec::new();
		let mut start = 0;
		let step_size = 1 << 9;

		let stream = match config.sample_format() {
			cpal::SampleFormat::F32 => {
				device
					.build_input_stream(
						&config.into(),
						move |data: &[f32], _: &_| {
							buf.extend_from_slice(data);
							while buf.len() - start > WINDOW_SIZE {
								ascii_display(&fft_spectrum(
									&mut (&buf[start..start + WINDOW_SIZE]).to_vec(),
								));
								start += step_size;
							}
							if start > 0 && (start > 4096 || start * 2 > buf.len()) {
								buf.drain(..start);
								start = 0;
							}
						},
						err_fn,
						None,
					)
					.unwrap()
			},
			sample_format => {
				panic!("Unsupported sample format '{sample_format}'")
			},
		};

		let _ = stream.play();
		thread::sleep(Duration::from_millis(1_000_000));
		drop(stream);
	}

	pub fn mic_into_pixels() {
		wavez::player::mic_into_pixels();
	}

	pub fn wav_player_mmap(path: &Path) {
		static MMAP: OnceLock<Mmap> = OnceLock::new();
		{
			let fd = File::open(path).unwrap();
			#[cfg(unix)]
			let fd = fd.into_raw_fd();
			#[cfg(windows)]
			let fd = fd.into_raw_handle();

			// the lifetime of the mmap is not tied to the lifetime of the file descriptor it was
			// created from, so Mmap: 'static
			//
			// SAFETY: this is unsound; we have no reason to think that the file won't be removed
			// while we read it. But we can't do anything about this; libc flock(2) is not strong enough
			// to prevent this, and it's also not cross-platform. So we don't have much of a choice.
			// The memmap2 crate docs guarantee that if we violate this assumption, we will get a
			// SIGBUS (and thus the program will terminate), which means this doesn't violate the
			// "real" memory safety of this program.
			MMAP.set(unsafe { Mmap::map(fd) }.unwrap())
				.expect("the oncelock cannot be initialized yet");
		}
		// 'static :)
		let mmap: &'static [u8] = &*(*MMAP.get().expect("the oncelock was just initialized"));

		let MmapedRiffPcm {
			samples_per_second,
			channels,
			samples,
		} = from_mmap(mmap);

		// TODO: refactor all this below into something like fn(&'static [i16]) -> thread handle {}
		let host = cpal::default_host();

		#[cfg(not(target_os = "linux"))]
		let device = host.default_output_device().unwrap();
		#[cfg(target_os = "linux")]
		let device = host
			.output_devices()
			.unwrap()
			.find(|dev| dev.name().as_deref() == Ok("pipewire"))
			.unwrap();

		println!("using audio device named \"{}\"", device.name().unwrap());

		let config = StreamConfig {
			channels: channels as u16,
			sample_rate: SampleRate(samples_per_second),
			buffer_size: BufferSize::Default,
		};

		dbg!(&config);

		let mut samples_player = samples;
		let stream = device
			.build_output_stream(
				&config,
				move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
					if let Some((head, tail)) = samples_player.split_at_checked(data.len()) {
						data.copy_from_slice(head);
						samples_player = tail;
						println!("{}", data.len());
					} else {
						data[..samples_player.len()].copy_from_slice(samples_player);
						data[samples_player.len()..].fill(0);
						samples_player = &[];
						std::process::exit(0);
					}
				},
				move |e| panic!("encountered error: {e}"),
				None,
			)
			.unwrap();

		stream.play().unwrap();
		loop {
			std::thread::yield_now();
		}
	}
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

fn main() {
	// #[allow(unused)]
	// use std::fs::File;

	// const PATH: &str = "test_files/chopin.wav";

	// // demos::mic_input();
	// // demos::wav_player(File::open(PATH).unwrap());
	// // demos::wav_visualizer(File::open(PATH).unwrap());
	// // demos::mic_into_pixels();
	// demos::wav_player_mmap(Path::new(PATH));

	let file_path = args().skip(1).next().unwrap();
	let file_buf: &'static [u8] = mmap_file(Path::new(&*file_path));
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

// struct PlayerState {
//     pub playing:bool,
//     pub player_idx_in_samples:usize,
// }

fn audio_video_combined(file: &Path) {

	// let file_buf: &'static [u8] = mmap_file(file);
	// let (song_info, samples) = wav_parse(file_buf);
	// let (tx, rx) = channel();
	// spawn_paused_child_audio_thread(rx, samples, song_info);
	// run_window(tx, samples, song_info);

	// let device = set up cpal audio device

	// let (header, data) = fn parser::parse_header(data) -> io::Result<(WaveHeader, WaveData /*impl Read + Seek*/)>;

	// // note: buffer size of audio stream should be fixed to something like 1/10th
	// // of the sample rate so that play/pause/seek is responsive
	// set up cpal audio stream(header, device)

	// // ASSUMPTION: the range of data required by the the visualization thread is a
	// // superset of the data required by the audio player thread
	// // note: this is buffered and only should be written to every second ish? we will
	// // have to find good numbers though
	// let shared_samples: Arc<Mutex<(
	// 	PlayerState { Paused | Playing, player_idx_in_samples: usize},
	// 	samples: Box<[i16]>
	// )>> = ...;

	// // the pixels/winit thread is the "boss", it controls the audio player thread.
	// // When it receives play/pause/seek signal from user, it updates the shared_samples
	// // buffer's data and its play/pause/position data.
	// clone arc into pixels thread before creation
	// let pixels = BoxSlice2d<Rgba>; // and move it into the closure
	// // create pixels thread:
	// stuff(move || {
	// 	// a lot of work to be done here wrt input handling to modify the shared_samples
	// 	// buffer
	// 	match keycode {
	// 		leftarrow => recompute range
	// 		space => toggle pause
	// 		...
	// 	}

	// 	// // calls this to update shared buffer only when needed:
	// 	// // extremely inefficient API but can be improved easily later, make the
	// 	// // simple, quick, and dirty thing now
	// 	fn parser::sample_range(_: WaveData, _: Range<usize>) -> io::Result<Box<[i16]>>;
	// 	for range in each fft range {
	// 		// // then right after calls this to calculate new ffts on this range
	// 		fn fft::fft(shared_samples[range].clone());
	// 		// // then
	// 		// // (this api can in the future be made more efficient by making it
	// 		// // io::Read-style)
	// 		let rgbas = fn frequencies_to_rgba(&[f32]) -> Box<[Rgba]>
	// 		// // then
	// 		pixels[range].memcpy(rgbas);
	// 	}
	// })

	// // // "consumer" audio thread
	// // make cpal audio thread
	// xxx.yyyy(move |fill_this: &mut [i16]| {
	// 	// // logic: when playerstate = paused, fill output stream with 0s
	// 	// otherwise:
	// 	fill_this.memcpy(shared_samples[player_idx_in_samples.. + fill_this.len()])
	// 	player_idx_in_samples += fill_this.len();
	// });

	// loop {
	// 	// later allow someone hitting q to toggle something in the state, if so
	// 	// then break this loop
	// }
}
