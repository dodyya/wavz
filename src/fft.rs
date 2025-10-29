use crate::complex::*;
pub fn fft(a: &Vec<Cplx>) -> Vec<Cplx> {
    // dbg!(a);
    let n = a.len();
    if n <= 1 {
        return a.clone();
    }
    let principal = Cplx::nth_principal(n);
    let mut omega = Cplx::new(1f64, 0f64);
    let a_evens: Vec<Cplx> = a.iter().step_by(2).copied().collect();
    let a_odds: Vec<Cplx> = a.iter().skip(1).step_by(2).copied().collect();

    let y_evens = fft(&a_evens);
    let y_odds = fft(&a_odds);

    let mut y = vec![Cplx::new(0f64, 0f64); n];
    // dbg!(&y);
    for k in 0..n / 2 {
        y[k] = y_evens[k] + omega * y_odds[k];
        y[k + n / 2] = y_evens[k] - omega * y_odds[k];
        omega *= principal;
    }
    return y;
}
