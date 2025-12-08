use crate::fft::Float;
use crate::fft::{BoxSlice2D, Slice2D};
use crate::rgba::*;

// TODO: figure out if theres an easier way

pub fn gen_spectrogram(spectra: Slice2D<Float>, range: f32) -> BoxSlice2D<Rgba> {
	let mut img = vec![Rgba::BLACK; spectra.width * spectra.height];

	let width = spectra.height;
	let height = spectra.width;
	gen_spectrogram_into(&mut img, spectra, range);

	let out = BoxSlice2D {
		width, //TRANSPOSE
		height,
		data: img.into_boxed_slice(),
	};
	out
}

pub fn gen_spectrogram_into(out: &mut [Rgba], spectra: Slice2D<Float>, range: f32) {
	let n_spectra = spectra.height;
	let n_rows_visible = out.len() / n_spectra;
	let n_rows_invisible = spectra.width - n_rows_visible; // Yeah this is shitty and tight coupling and this logic needs to go elsewhere.

	for x in 0..n_spectra {
		let spectrum = spectra.row(x);
		for (y, rgba) in render_spectrum_iter(spectrum, range)
			.skip(n_rows_invisible)
			.enumerate()
		{
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
