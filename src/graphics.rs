use crate::fft::Float;
use crate::fft::{BoxSlice2D, Slice2D};
use crate::rgba::*;

// TODO: figure out if theres an easier way

pub fn gen_spectrogram(spectra: BoxSlice2D<Float>, range: f32) -> BoxSlice2D<Rgba> {
	let mut img = vec![Rgba::BLACK; spectra.width * spectra.height];

	gen_spectrogram_into(&mut img, spectra.unbox(), range);

	BoxSlice2D {
		width: spectra.height, //TRANSPOSE
		height: spectra.width as usize,
		data: img.into_boxed_slice(),
	}
}

pub fn gen_spectrogram_into(out: &mut [Rgba], spectra: Slice2D<Float>, range: f32) {
	// println!(
	// 	"Generating spectrogram: w:{}, h:{}",
	// 	spectra.width, spectra.height,
	// );

	//We are transposing the spectrogram. Spectra is a "slice of slices", where [   ][   ][   ] they're concatenated like that.
	// Thus, its height is the number of spectrums, and its width is SPECTRUM_SIZE.
	let n_spectra = spectra.height;
	for x in 0..n_spectra {
		let spectrum = spectra.row(x);
		for (y, rgba) in render_spectrum_iter(spectrum, range).enumerate() {
			let start = y * n_spectra + x;
			out[start] = rgba;
		}
	}
}

pub fn render_spectrum(spectrum: &[f32], range: f32) -> Vec<Rgba> {
	render_spectrum_iter(spectrum, range).collect()
}

pub fn render_spectrum_iter(spectrum: &[f32], range: f32) -> impl Iterator<Item = Rgba> {
	// const RANGE: f32 = 0.005;
	const CUTOFF: f32 = 0.2; // Visual cutoff for what is black
	let growth = 1.001f32;

	spectrum.iter().scan(0.2f32, move |factor, &value| {
		let normed_hue = (*factor * (value) / range).clamp(0.0, 1.0);
		*factor = *factor * growth;
		if normed_hue > CUTOFF {
			Some(Rgba::hue(normed_hue))
		} else {
			Some(Rgba::BLACK)
		}
	})
}
