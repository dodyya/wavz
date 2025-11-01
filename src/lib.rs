pub mod complex;
pub mod fft;
pub mod parser;

#[cfg(test)]
mod tests {
	use std::fs::{File, read};

	use super::parser::{parse, parse1};

	// TODO: rename/remove irrelevant tests

	#[test]
	fn test_wav_read() {
		let file = read("./pure-tone.wav").unwrap();
		parse1(&*file);
	}

	#[test]
	fn test_big_wav_read() {
		let file = read("./choplin.wav").unwrap();
		parse1(&*file);
	}

	#[test]
	fn wav_read_real_parse() {
		let mut file = File::open("./pure-tone.wav").unwrap();

		parse(&mut file).unwrap();
	}
	#[test]
	fn big_wav_read_real_parse() {
		let mut file = File::open("./choplin.wav").unwrap();

		parse(&mut file).unwrap();
	}
}
