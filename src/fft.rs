use crate::complex::{Cplx, PI};

use fixed::FixedI16;
use fixed::types::extra::U15;

pub type Fix = FixedI16<U15>;
// TODO make fft impl not copy data when recursing; a possible solution is to map indices
// between iterations instead of copying the data each time.
//
// Ex: pass a fn(usize)->usize param and use it before indexing into slice,
// will be |x| 2*x+1 or |x| 2*x depending on recusion. Also transmit length, this
// might be complicated
pub fn copy_fft(a: &[Cplx]) -> Vec<Cplx> {
	let n = a.len();
	if n <= 1 {
		return a.to_vec();
	}

	let principal = Cplx::nth_principal(n);
	let mut omega = Cplx::new(1f32, 0f32);

	let a_evens: Vec<Cplx> = a.iter().step_by(2).copied().collect();
	let a_odds: Vec<Cplx> = a.iter().skip(1).step_by(2).copied().collect();

	let y_evens = copy_fft(&a_evens);
	let y_odds = copy_fft(&a_odds);

	let mut y = vec![Cplx::new(0f32, 0f32); n];

	// no idea what this does
	for k in 0..n / 2 {
		y[k] = y_evens[k] + omega * y_odds[k];
		y[k + n / 2] = y_evens[k] - omega * y_odds[k];
		omega *= principal;
	}

	y
}

/// Direct call to cosine and sine. O(u) roundoff.
pub fn roots_of_unity_1(n: usize) -> Vec<Cplx> {
	assert!(n.is_power_of_two()); //Check that n is a power of 2
	let theta = 2f32 * PI / n as f32;
	let mut out = Vec::<Cplx>::with_capacity(n / 2);
	for j in 0..n / 2 {
		out.push(Cplx::new(
			(j as f32 * theta).cos(),
			(j as f32 * -theta).sin(),
		));
	}
	out
}

/// Repeated multiplication. O(uj) roundoff.
pub fn roots_of_unity_2(n: usize) -> Vec<Cplx> {
	assert!(n.is_power_of_two()); //Check that n is a power of 2
	let mut out = Vec::<Cplx>::with_capacity(n / 2);
	out.push(Cplx::ONE);
	let theta = 2f32 * PI / n as f32;
	let omega = Cplx::new(theta.cos(), -theta.sin());
	for _ in 1..n / 2 {
		out.push(omega * *out.last().unwrap())
	}
	out
}

/// Subvector scaling
pub fn roots_of_unity_3(n: usize) -> Vec<Cplx> {
	assert!(n.is_power_of_two()); //Check that n is a power of 2
	let mut out = vec![Cplx::ZERO; n / 2];
	out[0] = Cplx::ONE;
	for j in 1..n.ilog2() as i32 {
		let two_j_m1 = 2usize.pow(j as u32 - 1);
		let mu = (1usize << j) as f32 * PI / n as f32;
		let omega = Cplx::new(mu.cos(), -mu.sin());
		for offset in 0usize..two_j_m1 {
			out[offset + two_j_m1] = omega * out[offset];
		}
	}

	out
}

/// Forward recursion
pub fn roots_of_unity_4(n: usize) -> Vec<Cplx> {
	assert!(n.is_power_of_two()); //Check that n is a power of 2
	let mut out = vec![Cplx::ZERO; n / 2];
	let theta = 2f32 * PI / n as f32;
	out[0] = Cplx::ONE;
	out[1] = Cplx::new(theta.cos(), -theta.sin());
	let tau = 2f32 * out[1].re;
	for j in 2..n / 2 {
		out[j].re = tau * out[j - 1].re - out[j - 2].re;
		out[j].im = tau * out[j - 1].im - out[j - 2].im;
	}

	out
}

/// Logarithmic Recursion
pub fn roots_of_unity_5(n: usize) -> Vec<Cplx> {
	assert!(n.is_power_of_two()); //Check that n is a power of 2
	let mut out = vec![Cplx::ZERO; n / 2];
	let theta = 2f32 * PI / n as f32;
	out[0] = Cplx::ONE;
	out[1] = Cplx::new(theta.cos(), -theta.sin());
	for k in 1..n.ilog2() {
		let p = 1 << (k as u32 - 1);
		out[p] = Cplx::new((p as f32 * theta).cos(), -(p as f32 * theta).sin());
		let tau = Cplx::new(2f32, 0f32) * out[p];
		for j in 1..p {
			out[p + j].re = tau.re * out[p - j].re - out[p - j].re;
			out[p + j].im = tau.im * out[p - j].re - out[p - j].im;
		}
	}
	out
}

pub fn roots_of_unity_6(n: usize) -> Vec<Cplx> {
	assert!(n.is_power_of_two());
	let q = n.ilog2() as usize;
	let lstar = n / 2;

	let theta = 2f32 * std::f32::consts::PI / n as f32;

	let mut c = vec![0f32; n];
	let mut s = vec![0f32; n];
	c[0] = 1.0;
	s[0] = 0.0;

	for k in 0..q {
		let p = 1usize << k;
		c[p] = (p as f32 * theta).cos();
		s[p] = -(p as f32 * theta).sin();
	}

	if q >= 2 {
		for lam in 1..=q - 2 {
			let p = 1usize << (q - lam - 2);
			let h = 1.0 / (2.0 * c[p]);
			let kmax = (1usize << lam) - 2;
			for k in 0..=kmax {
				let j = (3 + 2 * k) * p;
				c[j] = h * (c[j - p] + c[j + p]);
				s[j] = h * (s[j - p] + s[j + p]);
			}
		}
	}

	let mut out = Vec::with_capacity(lstar);
	for j in 0..lstar {
		out.push(Cplx::new(c[j], s[j]));
		// We assemble the output with 2 existing real
		// and imaginary vectors. Can SoA ComplexArray
		// pretty easily down the line.
	}
	out
}

// TODO: memoize somehow. Fix a constant num samples and gen sinewave once.
fn generate_sinewave(num_samples: usize) -> Vec<Fix> {
	let mut sinewave = Vec::with_capacity(num_samples);
	for i in 0..num_samples {
		sinewave.push(Fix::from_bits(
			(((i as f32 * std::f32::consts::TAU / num_samples as f32).sin()) * i32::MAX as f32)
				as i16,
		));
	}
	sinewave
}

/// Takes in imaginary slice as real and imaginary parts
pub fn fft_inplace(fr: &mut [Fix], fi: &mut [Fix]) {
	let num_samples: usize = fr.len();
	let sinewave = generate_sinewave(num_samples);
	let num_samples_m_1: usize = fr.len() - 1;
	let log2_num_samples = num_samples.ilog2() as usize;
	let shift_amt = 16i32 - log2_num_samples as i32;

	let mut tr: Fix; // temporary storage for swapping
	let mut ti: Fix;

	// let mut i: usize; // indices being combined in Danielson-Lanczos
	let mut j: usize;

	let mut l: usize; // Length of intermediate FFTs
	let mut k: isize; // Lookup trig values from sine table

	let mut istep: usize; // Length of the FFT result when you combine

	let mut wr: Fix; // Trigonometric values from lookup table
	let mut wi: Fix;

	let mut qr: Fix; // Temporary variables used during DL part of algorithm
	let mut qi: Fix;

	// m is one of indices being swapped
	let mut mr: usize; // the other index being swapped
	for m in 1..num_samples_m_1 {
		mr = ((m >> 1) & 0x5555) | ((m & 0x5555) << 1);
		// swap consecutive pairs
		mr = ((mr >> 2) & 0x3333) | ((mr & 0x3333) << 2);
		// swap nibbles ...
		mr = ((mr >> 4) & 0x0F0F) | ((mr & 0x0F0F) << 4);
		// swap bytes
		mr = ((mr >> 8) & 0x00FF) | ((mr & 0x00FF) << 8);
		// shift down mr
		mr >>= shift_amt;
		// don't swap that which has already been swapped
		if mr <= m {
			continue;
		};
		// swap the bit-reveresed indices
		// TODO: maybe do mem swap?
		tr = fr[m];
		fr[m] = fr[mr];
		fr[mr] = tr;
		ti = fi[m];
		fi[m] = fi[mr];
		fi[mr] = ti;
	}

	l = 1;
	k = log2_num_samples as isize - 1;

	while l < num_samples {
		istep = l << 1;
		for m in 0..l {
			j = m << k;
			// todo!();
			wr = sinewave[j + num_samples / 4];
			wi = -sinewave[j];
			wr >>= 1;
			wi >>= 1;

			for i in (m..num_samples).step_by(istep) {
				j = i + l;

				tr = wr * fr[j] - wi * fi[j];
				ti = wr * fi[j] + wi * fr[j];

				qr = fr[i] >> 1;
				qi = fi[i] >> 1;

				fr[j] = qr - tr;
				fi[j] = qi - ti;
				fr[i] = qr + tr;
				fi[i] = qi + ti;
			}
		}
		k -= 1;
		l = istep;
	}
}
