use fixed::FixedI16;
use fixed::types::extra::U15;

struct Cplx<T> {
	pub re: T,
	pub im: T,
}

pub type Fix = FixedI16<U15>;

// TODO: memoize somehow. Fix a constant num samples and gen sinewave once.
pub fn generate_sinewave(num_samples: usize) -> Vec<Fix> {
	let mut sinewave = Vec::with_capacity(num_samples);
	for i in 0..num_samples {
		sinewave.push(Fix::from_bits(
			(((i as f32 * std::f32::consts::TAU / num_samples as f32).sin()) * i16::MAX as f32)
				as i16,
		));
	}
	sinewave
}

/// Takes in a complex slice as real and imaginary parts, and
/// performs the FFT in-place. Magic.
pub fn fft_inplace(fr: &mut [Fix], fi: &mut [Fix]) {
	let n = fr.len();
	assert!(n.is_power_of_two() && fi.len() == n);

	let bits = n.ilog2() as u32;

	let num_samples: usize = fr.len();
	let sinewave: Vec<Fix> = generate_sinewave(num_samples);
	let log2_num_samples = num_samples.ilog2() as usize;

	for m in 1..n - 1 as usize {
		let mr = m.reverse_bits() >> (usize::BITS - bits);
		if mr > m {
			fr.swap(m, mr);
			fi.swap(m, mr);
		}
	}

	let mut temp_len = 1;
	let mut lookup = log2_num_samples as isize - 1;

	while temp_len < num_samples {
		let combined_len = temp_len << 1;
		for m in 0..temp_len {
			let mut j: usize = m << lookup;

			let w: Cplx<Fix> = Cplx {
				re: sinewave[j + num_samples / 4] / 2,
				im: -sinewave[j] / 2,
			};

			for i in (m..num_samples).step_by(combined_len) {
				j = i + temp_len;

				let t: Cplx<Fix> = Cplx {
					re: w.re * fr[j] - w.im * fi[j],
					im: w.re * fi[j] + w.im * fr[j],
				};

				let q = Cplx { re: fr[i] / 2, im: fi[i] / 2 };

				fr[j] = q.re - t.re;
				fi[j] = q.im - t.im;
				fr[i] = q.re + t.re;
				fi[i] = q.im + t.im;
			}
		}
		lookup -= 1;
		temp_len = combined_len;
	}
}
