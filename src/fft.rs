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

/// Takes in imaginary slice as real and imaginary parts, and
/// performs the FFT in-place. Magic.
pub fn fft_inplace(fr: &mut Vec<Fix>, fi: &mut Vec<Fix>) {
	#[inline]
	fn bit_rev(m: usize, mr: &mut usize) {
		//MAGIC
		*mr = ((m >> 1) & 0x5555) | ((m & 0x5555) << 1);
		*mr = ((*mr >> 2) & 0x3333) | ((*mr & 0x3333) << 2);
		*mr = ((*mr >> 4) & 0x0F0F) | ((*mr & 0x0F0F) << 4);
		*mr = ((*mr >> 8) & 0x00FF) | ((*mr & 0x00FF) << 8);
	}

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
	let mut mr: usize = 0; // the other index being swapped
	for m in 1..num_samples_m_1 {
		bit_rev(m, &mut mr);
		mr >>= shift_amt;
		// don't swap that which has already been swapped
		if mr <= m {
			continue;
		};
		// swap the bit-reveresed indices
		fr.swap(m, mr);
		fi.swap(m, mr);
	}

	l = 1;
	k = log2_num_samples as isize - 1;

	while l < num_samples {
		istep = l << 1;
		for m in 0..l {
			j = m << k;

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
