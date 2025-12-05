use std::io::{Read, Seek};
use std::path::Path;

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

fn main() {
	#[allow(unused)]
	use std::fs::File;

	const PATH: &str = "test_files/chopin.wav";

	// demos::mic_input();
	// demos::wav_player(File::open(PATH).unwrap());
	// demos::wav_visualizer(File::open(PATH).unwrap());
	// demos::mic_into_pixels();
	demos::wav_player_mmap(Path::new(PATH));
}

// struct PlayerState {
//     pub playing:bool,
//     pub player_idx_in_samples:usize,
// }

fn audio_video_combined(data: impl Read + Seek) {
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
