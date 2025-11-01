use crate::complex::{Cplx, PI};

// TODO make fft impl not copy data when recursing; a possible solution is to map indices
// between iterations instead of copying the data each time.
//
// Ex: pass a fn(usize)->usize param and use it before indexing into slice,
// will be |x| 2*x+1 or |x| 2*x depending on recusion. Also transmit length, this
// might be complicated
pub fn fft(a: &[Cplx]) -> Vec<Cplx> {
	let n = a.len();
	if n <= 1 {
		return a.to_vec();
	}

	let principal = Cplx::nth_principal(n);
	let mut omega = Cplx::new(1f32, 0f32);

	let a_evens: Vec<Cplx> = a.iter().step_by(2).copied().collect();
	let a_odds: Vec<Cplx> = a.iter().skip(1).step_by(2).copied().collect();

	let y_evens = fft(&a_evens);
	let y_odds = fft(&a_odds);

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
// TODO: Fix, because this totally doesn't work yet.
// Need [0..=m] calculation for roots_of_unity(n), not
// all of roots_of_unity(m). Not sure how to do that for recursive bisection.
pub fn symmetry_abuse_roots_of_unity(n: usize) -> Vec<Cplx> {
	if n < 8 {
		return roots_of_unity_6(n);
	}

	let m = n >> 3;

	let eighth = roots_of_unity_6(m);
	let mut out = Vec::with_capacity(n);
	out.extend_from_slice(&eighth);
	for i in (0..m).rev() {
		out.push(Cplx::new(-eighth[i].im, -eighth[i].re));
	}
	for i in 0..m {
		out.push(Cplx::new(eighth[i].im, -eighth[i].re));
	}
	for i in (0..m).rev() {
		out.push(Cplx::new(-eighth[i].re, eighth[i].im));
	}

	return out;
}
