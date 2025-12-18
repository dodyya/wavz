use std::ops::{Index, IndexMut};
use std::sync::LazyLock;

pub const WINDOW_SIZE: usize = 1 << 12; // 4096
pub const SPECTRUM_SIZE: usize = WINDOW_SIZE / 2;
pub const STEP_SIZE: usize = 1 << 8; // 256

struct Cplx<T> {
	pub re: T,
	pub im: T,
}

pub struct BoxSlice2D<T> {
	pub data: Box<[T]>,
	pub width: usize,
}

pub struct Slice2D<'a, T> {
	pub data: &'a [T],
	pub width: usize,
}

pub struct MutSlice2D<'a, T> {
	pub data: &'a mut [T],
	pub width: usize,
}

impl<T> Slice2D<'_, T> {
	pub fn row(&self, row: usize) -> &[T] {
		&self.data[row * self.width..(row + 1) * self.width]
	}
}

impl<T: Copy> Index<(usize, usize)> for Slice2D<'_, T> {
	type Output = T;

	fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
		&self.data[y * self.width + x]
	}
}

impl<T: Copy> Index<(usize, usize)> for MutSlice2D<'_, T> {
	type Output = T;

	fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
		&self.data[y * self.width + x]
	}
}

impl<T: Copy> IndexMut<(usize, usize)> for MutSlice2D<'_, T> {
	fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
		&mut self.data[y * self.width + x]
	}
}

impl<T> MutSlice2D<'_, T> {
	pub fn row(&self, row: usize) -> &[T] {
		&self.data[row * self.width..(row + 1) * self.width]
	}

	pub fn row_mut(&mut self, row: usize) -> &mut [T] {
		&mut self.data[row * self.width..(row + 1) * self.width]
	}
}

impl<T: Default + Copy> BoxSlice2D<T> {
	pub fn new(width: usize, height: usize) -> Self {
		// println!("{width}, {height}");
		BoxSlice2D {
			data: vec![Default::default(); width * height].into_boxed_slice(),
			width,
		}
	}

	pub fn row_mut(&mut self, row: usize) -> &mut [T] {
		&mut self.data[row * self.width..(row + 1) * self.width]
	}

	pub fn row(&self, row: usize) -> &[T] {
		&self.data[row * self.width..(row + 1) * self.width]
	}

	pub fn unbox(&self) -> Slice2D<'_, T> {
		Slice2D {
			data: self.data.as_ref(),
			width: self.width,
		}
	}

	pub fn unbox_mut(&mut self) -> MutSlice2D<'_, T> {
		MutSlice2D {
			data: self.data.as_mut(),
			width: self.width,
		}
	}
}

pub type Float = f32;

pub(crate) static SINE: LazyLock<Vec<Float>> = LazyLock::new(|| {
	let mut v = Vec::with_capacity(WINDOW_SIZE);
	for i in 0..WINDOW_SIZE {
		v.push((i as Float * std::f32::consts::TAU / WINDOW_SIZE as Float).sin());
	}
	v
});

/// Takes an fft result and returns the magnitude vector of the Nyquist range
fn spectrum(fr: &[Float], fi: &[Float]) -> Vec<Float> {
	let mut v = vec![0.0; WINDOW_SIZE / 2];
	spectrum_into(fr, fi, &mut v);
	v
}

fn spectrum_into(fr: &[Float], fi: &[Float], out: &mut [Float]) {
	for (e, i) in (0..WINDOW_SIZE / 2).rev().enumerate() {
		out[e] = fr[i].hypot(fi[i]);
	}
}

/// Takes only the real signal and performs fft + spectrum
pub fn fft_spectrum(real: &mut [Float]) -> Vec<Float> {
	// assert_eq!(RESOLUTION, v.len());
	let mut imag = vec![0.0; WINDOW_SIZE];
	fft_inplace(real, &mut imag);
	spectrum(&real, &imag)
}

pub fn fft_spectrum_into(input: &mut [Float], out: &mut [Float]) {
	let mut fi = vec![0.0; WINDOW_SIZE];
	fft_inplace(input, &mut fi);
	spectrum_into(input, &fi, out);
}

/// Takes in a complex slice as real and imaginary parts, and
/// performs the FFT in-place. Magic.
pub fn fft_inplace(fr: &mut [Float], fi: &mut [Float]) {
	assert_eq!(WINDOW_SIZE, fr.len());
	assert!(WINDOW_SIZE.is_power_of_two() && fi.len() == WINDOW_SIZE);

	let bits = WINDOW_SIZE.ilog2();

	let num_samples: usize = fr.len();
	let log2_num_samples = num_samples.ilog2() as usize;

	for m in 1..WINDOW_SIZE - 1 {
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

pub fn sliding_spectra(samples: &[f32]) -> BoxSlice2D<f32> {
	let num_ffts = (samples.len() - WINDOW_SIZE) / STEP_SIZE;
	let mut out = BoxSlice2D::<f32>::new(SPECTRUM_SIZE, num_ffts);
	sliding_spectra_into(samples, out.unbox_mut());

	out
}

pub fn sliding_spectra_into(samples: &[f32], mut out: MutSlice2D<f32>) {
	let num_ffts = (samples.len() - WINDOW_SIZE) / STEP_SIZE;
	let mut start = 0;
	let mut fr = Box::new([0.0; WINDOW_SIZE]);

	for i in 0..num_ffts {
		for j in 0..WINDOW_SIZE {
			fr[j] = samples[j + start];
		}

		fft_spectrum_into(fr.as_mut(), out.row_mut(i));
		start += STEP_SIZE;
	}
}
