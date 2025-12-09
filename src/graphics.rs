use crate::fft::Float;
use crate::fft::MutSlice2D;
use crate::fft::{BoxSlice2D, Slice2D};
use crate::rgba::*;

pub fn gen_spectrogram(spectra: Slice2D<Float>, range: f32) -> BoxSlice2D<Rgba> {
	let img_height = spectra.width; // TRANSPOSE!!
	let img_width = spectra.data.len() / img_height;
	let mut img = BoxSlice2D::<Rgba>::new(img_width, img_height);
	gen_spectrogram_into(spectra, range, img.unbox_mut());
	img
}

pub fn gen_spectrogram_into(spectra: Slice2D<Float>, sens: f32, out: MutSlice2D<Rgba>) {
	let n_spectra = spectra.data.len() / spectra.width;
	let n_rows_visible = out.data.len() / n_spectra;
	let n_rows_invisible = spectra.width - n_rows_visible; // Yeah this is shitty and tight coupling and this logic needs to go elsewhere.

	for x in 0..n_spectra {
		let spectrum = spectra.row(x);
		for (y, rgba) in render_spectrum_iter(spectrum, sens)
			.skip(n_rows_invisible)
			.enumerate()
		{
			let start = y * n_spectra + x;
			out.data[start] = rgba;
		}
	}
}

pub fn render_spectrum(spectrum: &[f32], sens: f32) -> Vec<Rgba> {
	render_spectrum_iter(spectrum, sens).collect()
}

pub fn render_spectrum_iter(spectrum: &[f32], sens: f32) -> impl Iterator<Item = Rgba> {
	// const RANGE: f32 = 0.005;
	const CUTOFF: f32 = 0.2; // Visual cutoff for what is black
	let growth = 1.001f32;

	spectrum.iter().scan(0.2f32, move |factor, &value| {
		let normed_hue = (*factor * (value) / sens).clamp(0.0, 1.0);
		*factor = *factor * growth;
		if normed_hue > CUTOFF {
			Some(Rgba::hue(normed_hue))
		} else {
			Some(Rgba::BLACK)
		}
	})
}
