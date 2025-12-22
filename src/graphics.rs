use crate::fft::{BoxSlice2D, MutSlice2D, SPECTRUM_SIZE, Slice2D};
use crate::rgba::*;

pub fn spectrogram(spectra: Slice2D<f32>, range: f32) -> BoxSlice2D<Rgba> {
	let img_height = spectra.width; // TRANSPOSE!!
	let img_width = spectra.data.len() / img_height;
	let mut img = BoxSlice2D::<Rgba>::new(img_width, img_height);
	spectrogram_into(spectra, range, img.unbox_mut());
	img
}

pub fn spectrogram_into(spectra: Slice2D<f32>, sens: f32, mut out: MutSlice2D<Rgba>) {
	let n_spectra = spectra.data.len() / SPECTRUM_SIZE;
	let n_rows_visible = out.data.len() / n_spectra;
	let n_rows_invisible = SPECTRUM_SIZE - n_rows_visible;

	// Perhaps profile loop order.
	for y in 0..n_rows_visible {
		for x in 0..n_spectra {
			out[(x, y)] = render(spectra[(y + n_rows_invisible, x)], sens);
		}
	}
}

pub fn render_spectrum(spectrum: &[f32], sens: f32) -> Vec<Rgba> {
	spectrum
		.iter()
		.map(move |&value| render(value, sens))
		.collect()
}

pub fn render(value: f32, sens: f32) -> Rgba {
	const CUTOFF: f32 = 0.2; // Visual cutoff for what is black
	let normed_hue = (value / sens).clamp(0.0, 1.0);
	if normed_hue > CUTOFF { Rgba::hue(normed_hue) } else { Rgba::BLACK }
}

pub fn draw_vbar(x: usize, mut out: MutSlice2D<Rgba>) {
	for y in 0..out.data.len() / out.width {
		out[(x, y)] = Rgba::WHITE;
	}
}
