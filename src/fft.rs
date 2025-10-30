use crate::complex::Cplx;

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
	let mut omega = Cplx::new(1f64, 0f64);

	let a_evens: Vec<Cplx> = a.iter().step_by(2).copied().collect();
	let a_odds: Vec<Cplx> = a.iter().skip(1).step_by(2).copied().collect();

	let y_evens = fft(&a_evens);
	let y_odds = fft(&a_odds);

	let mut y = vec![Cplx::new(0f64, 0f64); n];

	// no idea what this does
	for k in 0..n / 2 {
		y[k] = y_evens[k] + omega * y_odds[k];
		y[k + n / 2] = y_evens[k] - omega * y_odds[k];
		omega *= principal;
	}

	y
}
