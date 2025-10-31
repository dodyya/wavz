use std::f64::consts::PI;
use std::fmt::{self, Display};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

// TODO remove + inline once the IR format is decided
type Float = f64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cplx {
	pub re: Float,
	pub im: Float,
}

impl Cplx {
	const I: Cplx = Cplx { re: 0.0, im: 1.0 };
	const ONE: Cplx = Cplx { re: 1.0, im: 0.0 };
	const ZERO: Cplx = Cplx { re: 0.0, im: 0.0 };

	pub fn new(re: Float, im: Float) -> Self { Cplx { re, im } }

	pub fn exp(&self) -> Cplx {
		let r = self.re.exp();
		Cplx::new(r * self.im.cos(), r * self.im.sin())
	}

	pub fn mul_i(&self) -> Cplx { Cplx::new(-self.im, self.re) }

	pub fn div_i(&self) -> Cplx { Cplx::new(self.im, -self.re) }

	pub fn conj(&self) -> Cplx { Cplx::new(self.re, -self.im) }

	pub fn nth_principal(n: usize) -> Cplx { Self::exp(&Self::new(0.0, 2.0 * PI / n as Float)) }

	pub fn abs(&self) -> Float { (self.re * self.re + self.im * self.im).sqrt() }
}

impl Add for Cplx {
	type Output = Cplx;

	fn add(self, other: Cplx) -> Cplx { Cplx::new(self.re + other.re, self.im + other.im) }
}

impl AddAssign for Cplx {
	fn add_assign(&mut self, other: Cplx) {
		self.re += other.re;
		self.im += other.im;
	}
}

impl Sub for Cplx {
	type Output = Cplx;

	fn sub(self, other: Cplx) -> Cplx { Cplx::new(self.re - other.re, self.im - other.im) }
}

impl SubAssign for Cplx {
	fn sub_assign(&mut self, other: Cplx) {
		self.re -= other.re;
		self.im -= other.im;
	}
}

impl Mul for Cplx {
	type Output = Cplx;

	fn mul(self, other: Cplx) -> Cplx {
		Cplx::new(
			self.re * other.re - self.im * other.im,
			self.re * other.im + self.im * other.re,
		)
	}
}

impl MulAssign for Cplx {
	fn mul_assign(&mut self, other: Cplx) {
		let re = self.re * other.re - self.im * other.im;
		let im = self.re * other.im + self.im * other.re;
		self.re = re;
		self.im = im;
	}
}

impl Div for Cplx {
	type Output = Cplx;

	fn div(self, other: Cplx) -> Cplx {
		let denom = other.re * other.re + other.im * other.im;
		Cplx::new(
			(self.re * other.re + self.im * other.im) / denom,
			(self.im * other.re - self.re * other.im) / denom,
		)
	}
}

impl DivAssign for Cplx {
	fn div_assign(&mut self, other: Cplx) {
		let denom = other.re * other.re + other.im * other.im;
		let re = (self.re * other.re + self.im * other.im) / denom;
		let im = (self.im * other.re - self.re * other.im) / denom;
		self.re = re;
		self.im = im;
	}
}

impl Display for Cplx {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:.2} + {:.2}i", self.re, self.im)
	}
}

impl Neg for Cplx {
	type Output = Cplx;

	fn neg(self) -> Cplx { Cplx::new(-self.re, -self.im) }
}
