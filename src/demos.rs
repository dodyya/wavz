use std::env::args;
use std::fs::File;
#[cfg(unix)]
use std::os::fd::IntoRawFd;
#[cfg(windows)]
use std::os::windows::io::IntoRawHandle;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleRate, StreamConfig};
use memmap2::Mmap;

use crate::fft::{STEP_SIZE, fft_spectrum};
// pub use crate::audio_vis::audio_vis;
pub use crate::audio_vis::realtime_vis;
pub use crate::mic_vis::mic_vis;
use crate::parser::{MmapedRiffPcm, RiffWavePcm, from_mmap};
// pub fn mic_vis() {
// 	crate::mic_vis::mic_vis();
// }

pub fn precomp_vis(file_path: &str) {
	let data = File::open(file_path).unwrap();
	let data = RiffWavePcm::parse(data).unwrap();

	crate::precomp_vis::precomp_vis(data);
}

pub fn mic_ascii() {
	use crate::fft::WINDOW_SIZE;
	fn ascii_display(spectrum: &[f32]) {
		let mut buf = String::new();
		for x in spectrum.chunks_exact(14).rev() {
			let max_amp = x.iter().fold(0.0f32, |acc, &x| acc.max(x));
			buf.push_str(match max_amp {
				(..0.0001) => " ",
				(..0.0002) => ".",
				(..0.0004) => "+",
				(..0.0006) => "*",
				(..0.0010) => "#",
				(..0.0020) => "$",
				_ => "@",
			});
		}
		println!("{buf}");
	}

	let host = cpal::default_host();
	let device = host.default_input_device().unwrap();
	let config = device.default_input_config().unwrap();
	println!("{:?}", config);
	let err_fn = move |err| {
		eprintln!("an error occurred on stream: {err}");
	};

	let mut buf = Vec::new();
	let mut start = 0;

	let stream = match config.sample_format() {
		cpal::SampleFormat::F32 => device
			.build_input_stream(
				&config.into(),
				move |data: &[f32], _: &_| {
					buf.extend_from_slice(data);
					while buf.len() - start > WINDOW_SIZE {
						ascii_display(&fft_spectrum(
							&mut (&buf[start..start + WINDOW_SIZE]).to_vec(),
						));
						start += STEP_SIZE;
					}
					if start > 0 && (start > 4096 || start * 2 > buf.len()) {
						buf.drain(..start);
						start = 0;
					}
				},
				err_fn,
				None,
			)
			.unwrap(),
		sample_format => {
			panic!("Unsupported sample format '{sample_format}'")
		},
	};

	let _ = stream.play();
	thread::sleep(Duration::from_millis(1_000_000));
	drop(stream);
}

pub fn wav_player(path: &str) {
	static MMAP: OnceLock<Mmap> = OnceLock::new();
	{
		let fd = File::open(path).unwrap();
		#[cfg(unix)]
		let fd = fd.into_raw_fd();
		#[cfg(windows)]
		let fd = fd.into_raw_handle();

		// the lifetime of the mmap is not tied to the lifetime of the file descriptor it was
		// created from, so Mmap: 'static
		//
		// SAFETY: this is unsound; we have no reason to think that the file won't be removed
		// while we read it. But we can't do anything about this; libc flock(2) is not strong enough
		// to prevent this, and it's also not cross-platform. So we don't have much of a choice.
		// The memmap2 crate docs guarantee that if we violate this assumption, we will get a
		// SIGBUS (and thus the program will terminate), which means this doesn't violate the
		// "real" memory safety of this program.
		MMAP.set(unsafe { Mmap::map(fd) }.unwrap())
			.expect("the oncelock cannot be initialized yet");
	}
	// 'static :)
	let mmap: &'static [u8] = &*(*MMAP.get().expect("the oncelock was just initialized"));

	let MmapedRiffPcm {
		samples_per_second,
		channels,
		samples,
	} = from_mmap(mmap);

	// TODO: refactor all this below into something like fn(&'static [i16]) -> thread handle {}
	let host = cpal::default_host();

	#[cfg(not(target_os = "linux"))]
	let device = host.default_output_device().unwrap();
	#[cfg(target_os = "linux")]
	let device = host
		.output_devices()
		.unwrap()
		.find(|dev| dev.name().as_deref() == Ok("pipewire"))
		.unwrap();

	let config = StreamConfig {
		channels: channels as u16,
		sample_rate: SampleRate(samples_per_second),
		buffer_size: BufferSize::Default,
	};

	let mut samples_player = samples;
	let stream = device
		.build_output_stream(
			&config,
			move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
				if let Some((head, tail)) = samples_player.split_at_checked(data.len()) {
					data.copy_from_slice(head);
					samples_player = tail;
				} else {
					data[..samples_player.len()].copy_from_slice(samples_player);
					data[samples_player.len()..].fill(0);
					samples_player = &[];
					std::process::exit(0);
				}
			},
			move |e| panic!("encountered error: {e}"),
			None,
		)
		.unwrap();

	stream.play().unwrap();
	loop {
		std::thread::yield_now();
	}
}
