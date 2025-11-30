use std::sync::LazyLock;

pub const RESOLUTION: usize = 1 << 12; // 4096

struct Cplx<T> {
	pub re: T,
	pub im: T,
}

pub struct BoxSlice2D<T> {
	pub data: Box<[T]>,
	pub width: usize,
	pub height: usize,
}

impl<T: Default + Copy> BoxSlice2D<T> {
	pub fn new(width: usize, height: usize) -> Self {
		BoxSlice2D {
			data: vec![Default::default(); width * height].into_boxed_slice(),
			width,
			height,
		}
	}

	pub fn row_mut(&mut self, row: usize) -> &mut [T] {
		&mut self.data[row * self.width..(row + 1) * self.width]
	}

	pub fn row(&self, row: usize) -> &[T] {
		&self.data[row * self.width..(row + 1) * self.width]
	}

	pub fn concatenate(&self, other: &Self) -> Self {
		assert_eq!(self.height, other.height);
		//dbg!(self.width, other.width, self.height, other.height);
		let mut out = Self::new(self.width + other.width, self.height);
		for i in 0..self.height {
			out.row_mut(i)[..self.width].copy_from_slice(self.row(i));
			out.row_mut(i)[self.width..].copy_from_slice(other.row(i));
		}
		out
	}

	pub fn drain_cols(&mut self, count: usize) {
		self.data = self
			.data
			.chunks_exact_mut(self.width)
			.map(|chunk| chunk[count..].to_owned())
			.flatten()
			.collect::<Vec<_>>()
			.into_boxed_slice();
		self.width -= count;
	}
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
fn spectrum(fr: &[Float], fi: &[Float]) -> Vec<Float> {
	let mut v = Vec::with_capacity(RESOLUTION / 2);
	spectrum_into(&mut v, fr, fi);
	v
}

fn spectrum_into(out: &mut [Float], fr: &[Float], fi: &[Float]) {
	for i in 0..RESOLUTION / 2 {
		out[i] = (fr[i] * fr[i] + fi[i] * fi[i]).sqrt();
	}
}

pub fn fft_spectrum(real: &mut [Float]) -> Vec<Float> {
	// assert_eq!(RESOLUTION, v.len());
	let mut imag = vec![0.0; RESOLUTION];
	fft_inplace(real, &mut imag);
	spectrum(&real, &imag)
}

pub fn fft_spectrum_into(out: &mut [Float], input: &mut [Float]) {
	let mut fi = vec![0.0; RESOLUTION];
	fft_inplace(input, &mut fi);
	spectrum_into(out, input, &fi);
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
