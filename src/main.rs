use std::thread;
use std::time::Duration;

use cpal::traits::*;
use wavez::fft;
fn main() {
	let host = cpal::default_host();

	let device = host.default_input_device().unwrap();

	println!("using audio device named \"{}\"", device.name().unwrap());
	let config = device.default_input_config().unwrap();
	println!("{:?}", config);

	let err_fn = move |err| {
		eprintln!("an error occurred on stream: {err}");
	};

	let stream = match config.sample_format() {
		cpal::SampleFormat::F32 => device
			.build_input_stream(
				&config.into(),
				move |data: &[f32], _: &_| {
					let mut fr = data.to_vec();
					let mut fi = vec![0f32; fft::RESOLUTION];
					fft::fft_inplace(&mut fr, &mut fi);
					display(fft::spectrum(&fr, &fi));
				},
				err_fn,
				None,
			)
			.unwrap(),
		sample_format => {
			panic!("Unsupported sample format '{sample_format}'")
		},
	};

	let _ = stream.play();
	thread::sleep(Duration::from_millis(1000000));
	drop(stream);
}

fn display(spectrum: Vec<f32>) {
	// Display the spectrum data here
	let mut printable: String = "".into();
	for x in spectrum.chunks_exact(3) {
		printable.push_str(if x[0] > 0.0001 { "X" } else { " " });
	}
	println!("{}", printable);
}
