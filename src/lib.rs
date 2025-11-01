pub mod complex;
pub mod fft;
pub mod parser;

#[cfg(test)]
mod tests {
	use std::cmp::max;
	use std::fs::{File, read};

	use crate::{
		complex::{Cplx, PI},
		fft::*,
	};

	use super::parser::{parse, parse1};

	// TODO: rename/remove irrelevant tests

	#[test]
	fn test_wav_read() {
		let file = read("./pure-tone.wav").unwrap();
		parse1(&*file);
	}

	#[test]
	fn test_big_wav_read() {
		let file = read("./choplin.wav").unwrap();
		parse1(&*file);
	}

	#[test]
	fn wav_read_real_parse() {
		let mut file = File::open("./pure-tone.wav").unwrap();

		parse(&mut file).unwrap();
	}
	#[test]
	fn big_wav_read_real_parse() {
		let mut file = File::open("./choplin.wav").unwrap();

		parse(&mut file).unwrap();
	}

	#[test]
	fn decompose_cos_sum() {
		let size = 2048;

		let mut cosine = Vec::<Cplx>::with_capacity(size);

		let component = |x: f32, freq: f32| (2.0 * PI * freq / size as f32 * x).cos();

		let combination = |x: f32| {
			// Kinda whatever random periodic function
			5.0 + component(x, 2.0)
				+ -1.0 * component(x, 9.0)
				+ component(x, 37.0)
				+ 5.0 * component(x, 17.0)
				+ 105.3 * component(x, 7.0)
		};
		for i in 0..size {
			cosine.push(Cplx::new(combination(i as f32), 0f32));
		}

		let frequencies = fft(&cosine);
		dbg!(frequencies[0].abs());

		assert!((frequencies[0].abs() / size as f32 - 5.0).abs() < 1e-4);
		assert!((frequencies[2].abs() / size as f32 - 0.50).abs() < 1e-4);
		assert!((frequencies[7].abs() / size as f32 - 52.65).abs() < 1e-4);
		assert!((frequencies[9].abs() / size as f32 - 0.50).abs() < 1e-4);
		assert!((frequencies[17].abs() / size as f32 - 2.50).abs() < 1e-4);
		assert!((frequencies[37].abs() / size as f32 - 0.50).abs() < 1e-4);
	}
	//TODO: Benchamrk: )
	#[test]
	fn compare_roots_of_unity_impls() {
		let n = 1 << 5;
		let roots1 = roots_of_unity_1(n);
		let roots2 = roots_of_unity_2(n);
		let roots3 = roots_of_unity_3(n);
		let roots4 = roots_of_unity_4(n);
		let roots5 = roots_of_unity_5(n);
		let roots6 = roots_of_unity_6(n);

		let roots = [roots1, roots2, roots3, roots4, roots5, roots6];

		let mut tolerance = [[0.0; 6]; 6];

		for i in 0..6 {
			for j in 0..6 {
				for k in 0..n / 2 {
					let diff = (roots[i][k] - roots[j][k]).abs();
					if diff > tolerance[i][j] {
						tolerance[i][j] = diff;
					}
				}
			}
		}
		for i in 0..6 {
			for j in 0..6 {
				print!("{:.4} ", tolerance[i][j]);
			}
			println!();
		}
	}
}
