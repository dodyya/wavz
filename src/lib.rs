pub mod demos;
pub mod fft;
pub mod graphics;
pub mod mic;
pub mod parser;
pub mod precomp;
pub mod realtime;
pub mod rgba;

#[cfg(test)]
mod tests {
	use std::fs::File;

	use super::parser::RiffWavePcm;
	use crate::fft::*;

	// TODO: rename/remove irrelevant tests
	// /// Info for debugging
	fn hz_to_fft_index(hz: f32, samples_per_second: u32) -> usize {
		(hz * WINDOW_SIZE as f32 / (samples_per_second as f32)).round() as usize
	}

	#[test]
	fn wav_read_and_parse() {
		let mut file = File::open("./test_files/pure-tone.wav").unwrap();

		let output = RiffWavePcm::parse(&mut file).unwrap();
		let (sps, samp) = dbg!(output.samples_per_second, output.samples.len());
		dbg!(samp / sps as usize);
		assert!(samp / sps as usize == 10);
	}

	#[test]
	fn big_wav_read_and_parse() {
		let mut file = File::open("./test_files/chopin.wav").unwrap();

		let output = RiffWavePcm::parse(&mut file).unwrap();
		let (sps, samp) = dbg!(output.samples_per_second, output.samples.len());
		dbg!(samp / sps as usize, 29 * 60 + 25);
		assert!(samp / sps as usize == 29 * 60 + 25);
	}

	#[test]
	fn sine_gen() {
		use crate::fft::{SINE, WINDOW_SIZE};
		assert!(SINE.len() == WINDOW_SIZE);
		assert!(SINE[0] < 1e-10);
		assert!((SINE[WINDOW_SIZE / 4] - 1.0).abs() < 1e-10);
		assert!(SINE[WINDOW_SIZE / 2] < 1e-10);
		assert!((SINE[3 * WINDOW_SIZE / 4] + 1.0).abs() < 1e-10);
		assert!(SINE[WINDOW_SIZE - 1] < 1e-10);
	}

	#[test]
	fn synthetic_decompose() {
		let size = WINDOW_SIZE;
		let mut re = Vec::<Float>::with_capacity(size);
		let mut im = Vec::<Float>::with_capacity(size);

		let component =
			|x: f32, freq: f32| (2.0 * std::f32::consts::PI * freq / size as f32 * x).cos();

		let combination = |x: f32| {
			// Kinda whatever random periodic function
			component(x, 2.0)
				+ component(x, 7.0)
				+ component(x, 9.0)
				+ component(x, 17.0)
				+ component(x, 37.0)
		};
		for i in 0..size {
			re.push(combination(i as Float));
			im.push(0 as Float);
		}

		fft_inplace(&mut re, &mut im);
		for j in 0..size / 2 {
			let r = re[j];
			let i = im[j];
			if r * r + i * i > 0.01 {
				println!("frequency {j} had magnitude {:.4} ", r * r + i * i);
			}
		}
	}

	#[test]
	fn tone_from_file() {
		use crate::parser::RiffWavePcm;

		let file = File::open("./test_files/800hz.wav").unwrap();
		let RiffWavePcm { samples, samples_per_second } = RiffWavePcm::parse(file).unwrap();
		let samples = &*Box::leak(samples);
		let samples = &samples[..WINDOW_SIZE];
		println!("{}", samples.len());
		let mut re: Vec<Float> = Vec::with_capacity(samples.len());
		let mut im: Vec<Float> = Vec::with_capacity(samples.len());

		for &sample in samples {
			re.push(sample as Float);
			im.push(0 as Float);
		}

		fft_inplace(&mut re, &mut im);
		let mut max_index: usize = 0;
		for j in 0..samples.len() / 2 {
			let r = re[j];
			let i = im[j];
			if r * r + i * i > re[max_index] * re[max_index] + im[max_index] * im[max_index] {
				max_index = j;
			}
			if r * r + i * i > 0.0001 {
				println!("frequency {j} had magnitude {:.4} ", r * r + i * i);
			}
		}

		assert!(max_index == 74);
		assert!(hz_to_fft_index(800.0, samples_per_second) == max_index);
	}

	#[test]
	fn piano_harmonics() {
		use crate::parser::RiffWavePcm;

		let file = File::open("./test_files/ode.wav").unwrap();
		let RiffWavePcm { samples, samples_per_second } = RiffWavePcm::parse(file).unwrap();
		let samples = &*Box::leak(samples);
		let samples = &samples[..WINDOW_SIZE];
		let mut re: Vec<Float> = Vec::with_capacity(samples.len());

		for &sample in samples {
			re.push(sample as Float);
		}

		let amplitude = fft_spectrum(&mut re);

		let mut argsort = (0..amplitude.len()).collect::<Vec<usize>>();
		argsort.sort_by(|&a, &b| amplitude[b].partial_cmp(&amplitude[a]).unwrap()); //Argsort in decreasing order
		let middle_e = hz_to_fft_index(329.628, samples_per_second);
		for &freq in argsort[..20].iter() {
			// Top 20ish strongest frequencies
			// Must all be harmonics of middle e -- either close to it, or an integer multiple of it
			assert!((1..10).any(|i| {
				println!("freq: {}, middle_e * i: {}", freq, middle_e * i);
				(freq as f32 - (middle_e * i) as f32).abs() < 10.0
			}));
		}
	}
}
