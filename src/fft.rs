use std::sync::LazyLock;
pub(crate) const RESOLUTION: usize = 1 << 12;

pub(crate) struct Cplx<T> {
	pub re: T,
	pub im: T,
}

pub type Float = f32;

pub(crate) static SINE: LazyLock<Vec<Float>> = LazyLock::new(|| {
	let mut v = Vec::with_capacity(RESOLUTION);
	for i in 0..RESOLUTION {
		v.push((i as Float * std::f32::consts::TAU / RESOLUTION as Float).sin());
	}
	v
});

/// Takes an fft result and returns the magnitude vector of the Nyquist range
pub(crate) fn spectrum(fr: &[Float], fi: &[Float]) -> Vec<Float> {
	assert_eq!(RESOLUTION, fr.len());
	assert!(RESOLUTION.is_power_of_two() && fi.len() == RESOLUTION);

	let mut v = Vec::with_capacity(RESOLUTION / 2);
	for i in 0..RESOLUTION / 2 {
		v.push((fr[i] * fr[i] + fi[i] * fi[i]).sqrt());
	}
	v
}

/// Takes in a complex slice as real and imaginary parts, and
/// performs the FFT in-place. Magic.
pub(crate) fn fft_inplace(fr: &mut [Float], fi: &mut [Float]) {
	assert_eq!(RESOLUTION, fr.len());
	assert!(RESOLUTION.is_power_of_two() && fi.len() == RESOLUTION);

	let bits = RESOLUTION.ilog2();

	let num_samples: usize = fr.len();
	let log2_num_samples = num_samples.ilog2() as usize;

	for m in 1..RESOLUTION - 1 {
		let mr = m.reverse_bits() >> (usize::BITS - bits);
		if mr > m {
			fr.swap(m, mr);
			fi.swap(m, mr);
		}
	}

	let mut temp_len = 1;
	let mut lookup = log2_num_samples as isize - 1;

	while temp_len < num_samples {
		let combined_len = temp_len * 2;
		for m in 0..temp_len {
			let mut j: usize = m << lookup;

			let w: Cplx<Float> = Cplx {
				re: SINE[j + num_samples / 4] / 2 as Float,
				im: -SINE[j] / 2 as Float,
			};

			for i in (m..num_samples).step_by(combined_len) {
				j = i + temp_len;

				let t: Cplx<Float> = Cplx {
					re: w.re * fr[j] - w.im * fi[j],
					im: w.re * fi[j] + w.im * fr[j],
				};

				let q = Cplx {
					re: fr[i] / 2 as Float,
					im: fi[i] / 2 as Float,
				};

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

pub(crate) fn sliding_spectra(samples: &[i16], step_size: usize) -> Vec<Vec<Float>> {
	let mut ffts = Vec::new();
	let mut start = 0;

	while start + RESOLUTION <= samples.len() {
		let mut fr = Vec::with_capacity(RESOLUTION);
		let mut fi = Vec::with_capacity(RESOLUTION);

		for &sample in &samples[start..start + RESOLUTION] {
			fr.push(sample as Float);
			fi.push(0 as Float);
		}

		fft_inplace(&mut fr, &mut fi);
		ffts.push(spectrum(&fr, &fi));

		start += step_size;
	}

	ffts
}
