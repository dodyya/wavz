mod complex;
mod fft;

use std::f64::consts::PI;

use complex::Cplx;

fn main() {
	let size = 2048;

	let mut sine = Vec::<Cplx>::with_capacity(size);

	let component = |x: f64, freq: f64| (2.0 * PI * freq / size as f64 * x).cos();

	let combination = |x: f64| {
		// Kinda whatever random periodic function
		5.0 + component(x, 2.0)
			+ -1.0 * component(x, 9.0)
			+ component(x, 37.0)
			+ 5.0 * component(x, 17.0)
			+ 105.3 * component(x, 7.0)
	};
	for i in 0..size {
		sine.push(Cplx::new(combination(i as f64), 0f64));
	}

	let frequencies = fft::fft(&sine);

	for (i, f) in frequencies[..size / 2].iter().enumerate() {
		if f.abs() > 0.0001 {
			println!("x_{}={:.2}", i, f.abs() / size as f64);
			// Extracting coefficients from function above, only given points
		}
	}
}
