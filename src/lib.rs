pub mod fft;
pub mod graphics;
pub mod parser;

#[cfg(test)]
mod tests {
	use std::fs::File;

	use super::parser::RiffWavePcm;
	use crate::fft::*;

	// TODO: rename/remove irrelevant tests

	#[test]
	fn wav_read_and_parse() {
		let mut file = File::open("./pure-tone.wav").unwrap();

		let output = RiffWavePcm::parse(&mut file).unwrap();
		let (sps, samp) = dbg!(output.samples_per_second, output.samples.len());
		dbg!(samp / sps as usize);
		assert!(samp / sps as usize == 10);
	}
	#[test]
	fn big_wav_read_and_parse() {
		let mut file = File::open("./chopin.wav").unwrap();

		let output = RiffWavePcm::parse(&mut file).unwrap();
		let (sps, samp) = dbg!(output.samples_per_second, output.samples.len());
		dbg!(samp / sps as usize, 29 * 60 + 25);
		assert!(samp / sps as usize == 29 * 60 + 25);
	}

	// #[test]
	// fn decompose_cos_sum() {
	// 	let size = 2048;

	// 	let mut cosine = Vec::<Cplx>::with_capacity(size);

	// 	let component = |x: f32, freq: f32| (2.0 * PI * freq / size as f32 * x).cos();

	// 	let combination = |x: f32| {
	// 		// Kinda whatever random periodic function
	// 		component(x, 2.0)
	// 			+ component(x, 7.0)
	// 			+ component(x, 9.0)
	// 			+ component(x, 17.0)
	// 			+ component(x, 37.0)
	// 	};
	// 	for i in 0..size {
	// 		cosine.push(Cplx::new(combination(i as f32), 0f32));
	// 	}

	// 	let frequencies = copy_fft(&cosine);
	// 	for j in 0..size / 2 {
	// 		let r = frequencies[j].re;
	// 		let i = frequencies[j].im;
	// 		if r * r + i * i > 0.2 {
	// 			println!("frequency {j} had magnitude {:.4} ", r * r + i * i);
	// 		}
	// 	}
	// }

	#[test]
	fn test_sine_gen() {
		let sine4 = generate_sinewave(4);
		assert!(sine4[0].abs() < 0.0001);
		assert!(sine4[2].abs() < 0.0001);
		assert!((sine4[1] - Fix::MAX).abs() < 0.0001);
		assert!((sine4[3] - Fix::MIN).abs() < 0.0001);

		let sine32 = generate_sinewave(32);
		dbg!(sine32); //visually inspect for resembling sine.
	}

	#[test]
	// NOTE: See how the inplace, i16 fixed point
	// version does not compute the frequencies as accurately as the copy version.
	// (In the sense that their relative magnitudes aren't the same)
	// This is because the inplace version has literally half of the bits.
	fn decompose_cos_sum_inplace() {
		let size = 1 << 11;
		let mut re = Vec::<Fix>::with_capacity(size);
		let mut im = Vec::<Fix>::with_capacity(size);

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
			re.push(Fix::from_bits(
				(combination(i as f32) * i16::MAX as f32 / 2.0) as i16,
			));
			im.push(Fix::ZERO);
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
	fn test_fixed_point() {
		assert_eq!(Fix::MIN.to_bits(), i16::MIN);
		assert_eq!(Fix::MAX.to_bits(), i16::MAX);
		assert_eq!(Fix::from_bits(0), Fix::ZERO);
		// Takeaway: use .from_bits(i16) to turn any i16 with an "equivalent" float in [-1, 1)
	}

	#[test]
	fn integration() {
		use crate::parser::RiffWavePcm;

		let file = File::open("800hz.wav").unwrap();
		let RiffWavePcm { samples, .. } = RiffWavePcm::parse(file).unwrap();
		let samples = &*Box::leak(samples); // ez borrow checker error fix
		let largest_pow_of_2 = samples.len().next_power_of_two() / 2;
		let samples = &samples[..largest_pow_of_2];
		println!("{}", samples.len());
		let mut re: Vec<Fix> = Vec::with_capacity(samples.len());
		let mut im: Vec<Fix> = Vec::with_capacity(samples.len());

		for &sample in samples {
			re.push(Fix::from_bits(sample));
			im.push(Fix::ZERO);
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

		assert!(max_index == 4755);

		// quick math: 262144 samples, audio is 800hz. 44.1k samples per second. Then,
		// each bin corresponds to a sinusoidal which has a frequency of the bin
		// index measured in units of cycles per frame. 4755 cycles/frame.
		// we have sample rate f_s = 44.1kHz. FFT size 262144. Then our bin spacing is 44100/262144
		// = 0.1682281494 Hz. Multiply that spacing by 4755 to get 799.9999....
		// Yayy!
	}
}
