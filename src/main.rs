mod demos {
	use std::io::{Read, Seek};
	use std::sync::atomic::{AtomicBool, Ordering};
	use std::thread;
	use std::time::Duration;

	use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
	use cpal::{BufferSize, SampleRate, StreamConfig};
	use wavez::fft;
	use wavez::graphics::{gen_spectrogram, show_spectrogram};
	use wavez::parser::RiffWavePcm;

	#[allow(unused)]
	pub fn wav_visualizer(data: impl Read + Seek) {
		let RiffWavePcm {
			samples,
			samples_per_second: smps,
		} = RiffWavePcm::parse(data).unwrap();

		let step_size = 1 << 8;
		let spectra = gen_spectrogram(&mut fft::sliding_spectra(&samples, step_size));
		show_spectrogram(spectra, smps / step_size as u32);
	}

	#[allow(unused)]
	pub fn wav_player(data: impl Read + Seek) {
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

		let RiffWavePcm { samples_per_second, samples } = RiffWavePcm::parse(data).unwrap();

		let config = StreamConfig {
			channels: 1,
			sample_rate: SampleRate(samples_per_second),
			buffer_size: BufferSize::Default,
		};

		dbg!(&config);

		let mut samples = &*Box::leak(samples); // ez borrow checker error fix

		let is_done = &*Box::leak(Box::new(AtomicBool::new(false)));

		let stream = device
			.build_output_stream(
				&config,
				move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
					if let Some((head, tail)) = samples.split_at_checked(data.len()) {
						data.copy_from_slice(head);
						samples = tail;
					} else {
						data[..samples.len()].copy_from_slice(samples);
						data[samples.len()..].fill(0);
						samples = &[];
						(is_done).store(true, Ordering::Relaxed);
					}
				},
				move |e| panic!("encountered error: {e}"),
				None,
			)
			.unwrap();

		stream.play().unwrap();

		while !is_done.load(Ordering::Relaxed) {}
	}

	fn display(spectrum: &[f32]) {
		// Display the spectrum data here
		let mut buf = String::new();
		for x in spectrum.chunks_exact(3) {
			buf.push_str(if x[0] > 0.0001 { "X" } else { " " });
		}
		println!("{buf}");
	}

	#[allow(unused)]
	pub fn mic_input() {
		let host = cpal::default_host();

		let device = host.default_input_device().unwrap();

		println!("using audio device named \"{}\"", device.name().unwrap());
		let config = device.default_input_config().unwrap();
		println!("{config:?}");

		let err_fn = move |err| {
			eprintln!("an error occurred on stream: {err}");
		};

		let stream = match config.sample_format() {
			cpal::SampleFormat::F32 => {
				device
					.build_input_stream(
						&config.into(),
						move |data: &[f32], _: &_| {
							let mut fr = data.to_vec();
							let mut fi = vec![0f32; fft::RESOLUTION];
							fft::fft_inplace(&mut fr, &mut fi);
							display(&fft::spectrum(&fr, &fi));
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
}

fn main() {
	#[allow(unused)]
	use std::fs::File;

	#[allow(unused)]
	const PATH: &str = "test_files/800hz.wav";

	demos::mic_input();
	// demos::wav_player(File::open(PATH).unwrap());
	// demos::wav_visualizer(File::open(PATH).unwrap());
}

/*
fn audio_video_combined(data: impl Read + Seek) {
	// let device = set up cpal audio device

	let (header, data) = fn parser::parse_header(data) -> io::Result<(WaveHeader, WaveData /*impl Read + Seek*/)>;

	// // note: buffer size of audio stream should be fixed to something like 1/10th
	// // of the sample rate so that play/pause/seek is responsive
	// set up cpal audio stream(header, device)

	// // ASSUMPTION: the range of data required by the the visualization thread is a
	// // superset of the data required by the audio player thread
	// // note: this is buffered and only should be written to every second ish? we will
	// // have to find good numbers though
	let shared_samples: Arc<Mutex<(
		PlayerState { Paused | Playing, player_idx_in_samples: usize},
		samples: Box<[i16]>
	)>> = ...;

	// // the pixels/winit thread is the "boss", it controls the audio player thread.
	// // When it receives play/pause/seek signal from user, it updates the shared_samples
	// // buffer's data and its play/pause/position data.
	// clone arc into pixels thread before creation
	let pixels = BoxSlice2d<Rgba>; // and move it into the closure
	// create pixels thread:
	stuff(move || {
		// a lot of work to be done here wrt input handling to modify the shared_samples
		// buffer
		match keycode {
			leftarrow => recompute range
			space => toggle pause
			...
		}

		// // calls this to update shared buffer only when needed:
		// // extremely inefficient API but can be improved easily later, make the
		// // simple, quick, and dirty thing now
		fn parser::sample_range(_: WaveData, _: Range<usize>) -> io::Result<Box<[i16]>>;
		for range in each fft range {
			// // then right after calls this to calculate new ffts on this range
			fn fft::fft(shared_samples[range].clone());
			// // then
			// // (this api can in the future be made more efficient by making it
			// // io::Read-style)
			let rgbas = fn frequencies_to_rgba(&[f32]) -> Box<[Rgba]>
			// // then
			pixels[range].memcpy(rgbas);
		}
	})

	// // "consumer" audio thread
	// make cpal audio thread
	xxx.yyyy(move |fill_this: &mut [i16]| {
		// // logic: when playerstate = paused, fill output stream with 0s
		// otherwise:
		fill_this.memcpy(shared_samples[player_idx_in_samples.. + fill_this.len()])
		player_idx_in_samples += fill_this.len();
	});

	loop {
		// later allow someone hitting q to toggle something in the state, if so
		// then break this loop
	}
}

*/
