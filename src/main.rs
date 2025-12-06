use wavez::demos::*;
fn main() {
	mic_vis();
	precomp_vis();
	audio_vis();
	mic_ascii();
	wav_player();
}

// fn audio_video_combined(file: &Path) {

// let file_buf: &'static [u8] = mmap_file(file);
// let (song_info, samples) = wav_parse(file_buf);
// let (tx, rx) = channel();
// spawn_paused_child_audio_thread(rx, samples, song_info);
// run_window(tx, samples, song_info);

// let device = set up cpal audio device

// let (header, data) = fn parser::parse_header(data) -> io::Result<(WaveHeader, WaveData /*impl Read + Seek*/)>;

// // note: buffer size of audio stream should be fixed to something like 1/10th
// // of the sample rate so that play/pause/seek is responsive
// set up cpal audio stream(header, device)

// // ASSUMPTION: the range of data required by the the visualization thread is a
// // superset of the data required by the audio player thread
// // note: this is buffered and only should be written to every second ish? we will
// // have to find good numbers though
// let shared_samples: Arc<Mutex<(
// 	PlayerState { Paused | Playing, player_idx_in_samples: usize},
// 	samples: Box<[i16]>
// )>> = ...;

// // the pixels/winit thread is the "boss", it controls the audio player thread.
// // When it receives play/pause/seek signal from user, it updates the shared_samples
// // buffer's data and its play/pause/position data.
// clone arc into pixels thread before creation
// let pixels = BoxSlice2d<Rgba>; // and move it into the closure
// // create pixels thread:
// stuff(move || {
// 	// a lot of work to be done here wrt input handling to modify the shared_samples
// 	// buffer
// 	match keycode {
// 		leftarrow => recompute range
// 		space => toggle pause
// 		...
// 	}

// 	// // calls this to update shared buffer only when needed:
// 	// // extremely inefficient API but can be improved easily later, make the
// 	// // simple, quick, and dirty thing now
// 	fn parser::sample_range(_: WaveData, _: Range<usize>) -> io::Result<Box<[i16]>>;
// 	for range in each fft range {
// 		// // then right after calls this to calculate new ffts on this range
// 		fn fft::fft(shared_samples[range].clone());
// 		// // then
// 		// // (this api can in the future be made more efficient by making it
// 		// // io::Read-style)
// 		let rgbas = fn frequencies_to_rgba(&[f32]) -> Box<[Rgba]>
// 		// // then
// 		pixels[range].memcpy(rgbas);
// 	}
// })

// // // "consumer" audio thread
// // make cpal audio thread
// xxx.yyyy(move |fill_this: &mut [i16]| {
// 	// // logic: when playerstate = paused, fill output stream with 0s
// 	// otherwise:
// 	fill_this.memcpy(shared_samples[player_idx_in_samples.. + fill_this.len()])
// 	player_idx_in_samples += fill_this.len();
// });

// loop {
// 	// later allow someone hitting q to toggle something in the state, if so
// 	// then break this loop
// }
// }
