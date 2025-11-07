pub mod complex;
pub mod fft;
pub mod parser;

#[cfg(test)]
mod tests {
	use std::cmp::max;
	use std::fs::{File, read};

	use super::parser::{parse, parse1};
	use crate::complex::{Cplx, PI};
	use crate::fft::{copy_fft, *};

	// TODO: rename/remove irrelevant tests

	#[test]
	fn test_wav_read() {
		let file = read("./pure-tone.wav").unwrap();
		parse1(&*file);
	}

	#[test]
	fn test_big_wav_read() {
		let file = read("./chopin.wav").unwrap();
		parse1(&*file);
	}

	#[test]
	fn wav_read_real_parse() {
		let mut file = File::open("./pure-tone.wav").unwrap();

		let output = parse(&mut file).unwrap();
		let (sps, samp) = dbg!(output.samples_per_second, output.samples.len());
		dbg!(samp / sps as usize);
		assert!(samp / sps as usize == 40);
	}
	#[test]
	fn big_wav_read_real_parse() {
		let mut file = File::open("./chopin.wav").unwrap();

		let output = parse(&mut file).unwrap();
		let (sps, samp) = dbg!(output.samples_per_second, output.samples.len());
		dbg!(samp / sps as usize, 29 * 60 + 25);
		assert!(samp / sps as usize == 29 * 60 + 25);
	}

	#[test]
	fn decompose_cos_sum() {
		let size = 2048;

		let mut cosine = Vec::<Cplx>::with_capacity(size);

		let component = |x: f32, freq: f32| (2.0 * PI * freq / size as f32 * x).cos();

		let combination = |x: f32| {
			// Kinda whatever random periodic function
			component(x, 2.0)
				+ component(x, 7.0)
				+ component(x, 9.0)
				+ component(x, 17.0)
				+ component(x, 37.0)
		};
		for i in 0..size {
			cosine.push(Cplx::new(combination(i as f32), 0f32));
		}

		let frequencies = copy_fft(&cosine);
		for j in 0..size / 2 {
			let r = frequencies[j].re;
			let i = frequencies[j].im;
			if r * r + i * i > 0.2 {
				println!("frequency {j} had magnitude {:.4} ", r * r + i * i);
			}
		}
	}

	#[test]
	fn decompose_cos_sum_inplace() {
		let size = 1 << 11;
		let mut re = Vec::<Fix>::with_capacity(size);
		let mut im = Vec::<Fix>::with_capacity(size);

		let component = |x: f32, freq: f32| (2.0 * PI * freq / size as f32 * x).cos();

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
			if r * r + i * i > 0.2 {
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
}
