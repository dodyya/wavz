use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};
use cpal::{BufferSize, SampleRate, StreamConfig};
use pixels::wgpu::core::resource::ResourceInfo;
// use wavez::fft::sliding_fft;
// use wavez::graphics::draw_fft;
use wavez::lib::run_demo;
use wavez::parser::RiffWavePcm;

fn main() {
	run_demo("./test_files/ode.wav");

	// println!("we made {} ffts", ffts.len());
	// println!("each fft has {} frequency samples", ffts[0].re.len());
	// println!(
	// 	"The length of the song is {} seconds",
	// 	samples.len() as f64 / samples_per_second as f64
	// );
	// let fft_period = samples.len() as f64 / samples_per_second as f64 / ffts.len() as f64;
	// println!(
	// 	"Each fft corresponds to {} seconds",
	// 	samples.len() as f64 / 44100.0 / ffts.len() as f64
	// );
	// draw_fft(&ffts, fft_period);
	// smain();

	// let host = cpal::default_host();

	// #[cfg(not(target_os = "linux"))]
	// let device = host.default_output_device().unwrap();
	// #[cfg(target_os = "linux")]
	// let device = host
	// 	.output_devices()
	// 	.unwrap()
	// 	.find(|dev| dev.name().as_deref() == Ok("pipewire"))
	// 	.unwrap();

	// println!("using audio device named \"{}\"", device.name().unwrap());

	// let RiffWavePcm { samples_per_second, samples } = RiffWavePcm::parse(file).unwrap();

	// let config = StreamConfig {
	// 	channels: 1,
	// 	sample_rate: SampleRate(samples_per_second),
	// 	buffer_size: BufferSize::Default,
	// };

	// dbg!(&config);

	// let mut samples = &*Box::leak(samples); // ez borrow checker error fix

	// let is_done = &*Box::leak(Box::new(AtomicBool::new(false)));

	// let stream = device
	// 	.build_output_stream(
	// 		&config,
	// 		move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
	// 			if let Some((head, tail)) = samples.split_at_checked(data.len()) {
	// 				data.copy_from_slice(head);
	// 				samples = tail;
	// 			} else {
	// 				(&mut data[..samples.len()]).copy_from_slice(samples);
	// 				(&mut data[samples.len()..]).fill(0);
	// 				samples = &[];
	// 				(is_done).store(true, Ordering::Relaxed);
	// 			}
	// 		},
	// 		move |e| panic!("encountered error: {e}"),
	// 		None,
	// 	)
	// 	.unwrap();

	// stream.play().unwrap();

	// while !is_done.load(Ordering::Relaxed) {}
}
