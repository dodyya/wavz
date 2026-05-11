use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct ChunkHeaderRaw {
	tag: [u8; 4],
	num_bytes: [u8; 4],
}
impl ChunkHeaderRaw {
	fn into_chunk_header(self) -> ChunkHeader {
		ChunkHeader {
			tag: self.tag,
			num_bytes: u32::from_le_bytes(self.num_bytes),
		}
	}
}

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct ChunkHeader {
	tag: [u8; 4],
	num_bytes: u32,
}
impl ChunkHeader {
	const DATA_TAG: [u8; 4] = *b"data";
	const FORMAT_TAG: [u8; 4] = *b"fmt ";
	const RIFF_TAG: [u8; 4] = *b"RIFF";
}

// TODO this struct is indicative of poorly designed, non-rusty parsing code, but
// it would take a significant of work to fix
struct DataFormat;
impl DataFormat {
	const EXTENSIBLE: u16 = 0xfffe;
	const PCM: u16 = 0x0001;
}

#[derive(Clone, Copy, Debug)]
pub enum Channels {
	One = 1,
	Two = 2,
}

pub mod precomp {
	use core::error::Error;
	use core::fmt;
	use std::io::{self, SeekFrom};

	use bytemuck::{Pod, Zeroable, must_cast_mut, must_cast_slice_mut, zeroed_slice_box};

	use super::{ChunkHeader, DataFormat};

	#[non_exhaustive]
	enum FormatSizeConvError {
		IncorrectSize(u32),
	}
	impl fmt::Display for FormatSizeConvError {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			match self {
				Self::IncorrectSize(size) => {
					write!(
						f,
						"format chunk size of {size} is invalid. must be one of 16, 18, or 40",
					)
				},
			}
		}
	}
	impl fmt::Debug for FormatSizeConvError {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			<Self as fmt::Display>::fmt(self, f)
		}
	}
	impl Error for FormatSizeConvError {}

	#[repr(u32)]
	enum FormatSize {
		Size16 = Self::SIZE16,
		Size18 = Self::SIZE18,
		Size40 = Self::SIZE40,
	}
	impl FormatSize {
		const SIZE16: u32 = 16;
		const SIZE18: u32 = 18;
		const SIZE40: u32 = 40;

		const fn new(size: u32) -> Result<Self, FormatSizeConvError> {
			match size {
				Self::SIZE16 => Ok(Self::Size16),
				Self::SIZE18 => Ok(Self::Size18),
				Self::SIZE40 => Ok(Self::Size40),
				size => Err(FormatSizeConvError::IncorrectSize(size)),
			}
		}
	}
	#[derive(Clone, Copy, Pod, Zeroable, Debug)]
	#[repr(C)]
	struct RawRiffData {
		riff_header: ChunkHeader,
		wave_tag: [u8; 4],
		format_header: ChunkHeader,
		data_format: u16,
		num_channels: u16,
		sample_blocks_per_sec: u32,
		bytes_per_second: u32,
		sample_block_size: u16,
		bits_per_sample: u16,
	}
	impl RawRiffData {
		const WAVE_TAG: [u8; 4] = *b"WAVE";
	}

	#[derive(Clone, Copy, Pod, Zeroable, Debug)]
	#[repr(C)]
	struct WaveExtention {
		extension_size: u16, // 22
		valid_bits_per_sample: u16,
		speaker_position_mask: [u8; 4],
		data_format: [u8; 2],
		extention_tag: [u8; 14],
	}
	impl WaveExtention {
		const EXTENSION_SIZE: u16 = 22;
		const TAG: [u8; 14] = [
			0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
		];
	}

	pub struct RiffWavePcm {
		pub samples_per_second: u32,
		pub samples: Box<[i16]>,
	}
	impl RiffWavePcm {
		fn from_components(samples: Box<[i16]>, channels: usize, bytes_per_second: u32) -> Self {
			let conversion_factor = (channels * size_of::<i16>()) as u32;
			Self {
				samples_per_second: bytes_per_second / conversion_factor,
				samples,
			}
		}

		fn skip_until_data_chunk(source: impl io::Read + io::Seek) -> io::Result<ChunkHeader> {
			let mut source = source;

			loop {
				let mut header = ChunkHeader::zeroed();
				source.read_exact(must_cast_mut::<_, [_; 8]>(&mut header))?;

				if header.tag == ChunkHeader::DATA_TAG {
					return Ok(header);
				}

				source.seek(SeekFrom::Current(header.num_bytes as i64))?;
			}
		}

		fn parse_data_chunk(
			source: impl io::Read + io::Seek,
			samples: usize,
			channels: usize,
		) -> io::Result<Box<[i16]>> {
			// PERF: see how varying this constant affects performance
			const MAX_SAMPLE_BLOCKS: usize = 10_000_000;

			// PERF: see if specialization for len=2 yields benefits
			/// stolen from <https://github.com/ascent12/average/blob/master/avg.c>
			fn avg_perfect(slice: &[i16]) -> i16 {
				let n = slice.len() as i64;
				let mut avg: i16 = 0;

				let mut error = 0;

				for &val in slice {
					error += (val as i64 % n) as i16;
					avg += ((val as i64 / n) + (error as i64 / n)) as i16;
					error = (error as i64 % n) as i16;
				}

				if avg < 0 && error > 0 {
					avg += 1;
				} else if avg > 0 && error < 0 {
					avg -= 1;
				}

				avg
			}

			let mut source = source;
			let num_samples_per_channel = samples / channels;

			let mut buf = zeroed_slice_box::<i16>(num_samples_per_channel);

			if channels == 1 {
				source.read_exact(must_cast_slice_mut::<_, u8>(&mut buf))?;
			} else {
				let mut index = 0;
				let mut num_remaining_samples = samples;
				let mut temp_samples =
					zeroed_slice_box::<i16>(samples.min(MAX_SAMPLE_BLOCKS * channels));

				while num_remaining_samples > 0 {
					let num_samples_to_read = num_remaining_samples.min(temp_samples.len());
					let temp_samples_buf = &mut temp_samples[..num_samples_to_read];
					source.read_exact(must_cast_slice_mut::<_, u8>(temp_samples_buf))?;
					num_remaining_samples -= num_samples_to_read;

					// TODO make this zip an iterator of buf[slice..slice].iter_mut() and the chunks_exact
					for chunk in temp_samples_buf.chunks_exact(channels) {
						buf[index] = avg_perfect(chunk);
						index += 1;
					}
				}
			}

			Ok(buf)
		}

		pub fn parse(source: impl io::Read + io::Seek) -> io::Result<RiffWavePcm> {
			let mut source = source;
			let mut riff_data = RawRiffData::zeroed();

			source.read_exact(must_cast_mut::<_, [_; 36]>(&mut riff_data))?;

			if riff_data.riff_header.tag != ChunkHeader::RIFF_TAG {
				return Err(io::Error::other("chunk header must be \"RIFF\""));
			}
			if riff_data.wave_tag != RawRiffData::WAVE_TAG {
				return Err(io::Error::other("first bytes in header must be \"WAVE\""));
			}
			if riff_data.format_header.tag != ChunkHeader::FORMAT_TAG {
				return Err(io::Error::other("format tag must be \"fmt \""));
			}

			let format_size =
				FormatSize::new(riff_data.format_header.num_bytes).map_err(io::Error::other)?;

			// TODO: this is in dire need of some kind of abstraction
			// options may include some kind of (format size, data format) pair type since they depend on each other a lot
			match format_size {
				FormatSize::Size16 => {
					if riff_data.data_format != DataFormat::PCM {
						return Err(io::Error::other("data format must be the PCM data format"));
					}
				},
				FormatSize::Size18 => {
					if riff_data.data_format != DataFormat::PCM {
						return Err(io::Error::other("data format must be the PCM data format"));
					}

					let mut ext_size = <[u8; 2]>::zeroed();
					source.read_exact(&mut ext_size[..])?;

					if ext_size != [0x0, 0x00] {
						return Err(io::Error::other("extension size must be zero"));
					}
				},
				FormatSize::Size40 => {
					if riff_data.data_format != DataFormat::EXTENSIBLE {
						return Err(io::Error::other(
							"data format must be the extensible data format",
						));
					}

					let mut ext = WaveExtention::zeroed();
					source.read_exact(must_cast_mut::<_, [_; 24]>(&mut ext))?;
					if ext.extension_size != WaveExtention::EXTENSION_SIZE {
						return Err(io::Error::other("extension size must be 22"));
					}
					if ext.data_format != DataFormat::PCM.to_le_bytes() {
						return Err(io::Error::other("data format must be the PCM data format"));
					}
					if ext.extention_tag != WaveExtention::TAG {
						return Err(io::Error::other(
							"extension tag must be the wave extension tag",
						));
					}
				},
			}

			let ChunkHeader { num_bytes, .. } = RiffWavePcm::skip_until_data_chunk(&mut source)?;

			let num_samples = num_bytes as usize / size_of::<i16>();
			let channels = riff_data.num_channels as usize;

			let samples = RiffWavePcm::parse_data_chunk(&mut source, num_samples, channels)?;

			Ok(RiffWavePcm::from_components(
				samples,
				channels,
				riff_data.bytes_per_second,
			))
		}
	}
}

pub mod mmap {
	use std::fs::File;
	use std::sync::OnceLock;

	use bytemuck::{AnyBitPattern, cast_slice};
	use memmap2::Mmap;

	use super::{Channels, ChunkHeader, ChunkHeaderRaw, DataFormat};

	#[derive(Clone, Copy, Debug)]
	pub struct MmapedRiffPcm<'samples> {
		pub samples_per_second: u32,
		pub channels: Channels,
		pub samples: &'samples [i16],
	}

	#[derive(AnyBitPattern, Debug, Clone, Copy)]
	#[repr(C)]
	struct Format16 {
		data_format: u16,
		num_channels: u16,
		sample_blocks_per_sec: u32,
		bytes_per_second: u32,
		sample_block_size: u16,
		bits_per_sample: u16,
	}
	#[derive(AnyBitPattern, Debug, Clone, Copy)]
	#[repr(C)]
	struct Format18 {
		data_format: u16,
		num_channels: u16,
		sample_blocks_per_sec: [u8; 4],
		bytes_per_second: [u8; 4],
		sample_block_size: u16,
		bits_per_sample: u16,
		extension_size: u16, // could validate: 0
	}
	#[derive(AnyBitPattern, Debug, Clone, Copy)]
	#[repr(C)]
	struct Format40 {
		data_format_ext: u16, // could validate: extensible
		num_channels: u16,
		sample_blocks_per_sec: u32,
		bytes_per_second: u32,
		sample_block_size: u16,
		bits_per_sample: u16,
		extension_size: u16, // could validate: 22
		valid_bits_per_sample: u16,
		speaker_position_mask: [u8; 4],
		data_format: u16,
		extention_tag: [u8; 14],
	}

	pub fn mmap_file(path: &str) -> &'static [u8] {
		static MMAP: OnceLock<Mmap> = OnceLock::new();

		{
			let file = File::open(path).unwrap();

			// the lifetime of the mmap is not tied to the lifetime of the file descriptor it was
			// created from, so Mmap: 'static
			//
			// SAFETY: this is unsound; we have no reason to think that the file won't be [re]moved
			// while we read it. But we can't do anything about this; libc flock(2) is not strong enough
			// to prevent this, and it's also not cross-platform. So we don't have much of a choice.
			// the memmap2 crate docs guarantee that if we violate this assumption, we will get a
			// SIGBUS (and thus the program will terminate), which means this doesn't violate the
			// "real" memory safety of this program; we have committed library UB and that is the
			// best we can get.
			MMAP.set(unsafe { Mmap::map(&file) }.unwrap())
				.expect("the oncelock has not yet been initialized");
		}
		// 'static :)
		&*(*MMAP.get().expect("the oncelock was just initialized"))
	}

	/// splits &[u8] into (&T, &[u8])
	/// panics if unaligned or not enough bytes for a T in data
	macro_rules! split_cast_rem {
		($var:ident, $type:ty) => {
			(
				::bytemuck::cast_ref::<[u8; { ::core::mem::size_of::<$type>() }], $type>(
					$var.first_chunk::<{ ::core::mem::size_of::<$type>() }>()
						.unwrap(),
				),
				&$var[{ ::core::mem::size_of::<$type>() }..],
			)
		};
	}

	/// panics if not enough data or if unaligned (will not happen if the slice
	/// is actually from mmap, which returns page-aligned slices)
	pub fn from_mmap(file_data: &[u8]) -> MmapedRiffPcm<'_> {
		let (riff_header, rest) = split_cast_rem!(file_data, ChunkHeaderRaw);
		assert!(riff_header.tag == ChunkHeader::RIFF_TAG);

		let (wav_tag, rest) = split_cast_rem!(rest, [u8; 4]);
		assert!(wav_tag == b"WAVE");

		let (fmt_header, rest) = split_cast_rem!(rest, ChunkHeaderRaw);
		let fmt_header = fmt_header.into_chunk_header();
		assert!(fmt_header.tag == ChunkHeader::FORMAT_TAG);

		let (bytes_per_sec, num_channels, data_format, rest) = match fmt_header.num_bytes {
			16 => {
				let (d, rest) = split_cast_rem!(rest, Format16);
				(d.bytes_per_second, d.num_channels, d.data_format, rest)
			},
			18 => {
				let (d, rest) = split_cast_rem!(rest, Format18);
				(
					u32::from_le_bytes(d.bytes_per_second),
					d.num_channels,
					d.data_format,
					rest,
				)
			},
			40 => {
				let (d, rest) = split_cast_rem!(rest, Format40);
				(d.bytes_per_second, d.num_channels, d.data_format, rest)
			},
			wrong => {
				panic!("chunk size of `{wrong}` is not 16, 18, or 40");
			},
		};

		assert!(data_format == DataFormat::PCM);
		let channels = match num_channels {
			1 => Channels::One,
			2 => Channels::Two,
			n => panic!("right now only 1 or 2 channels is supported, which {n} is not"),
		};

		let mut data = rest;

		// find data chunk
		let size = loop {
			let (header, rest) = split_cast_rem!(data, ChunkHeaderRaw);
			let header = header.into_chunk_header();
			let size = header.num_bytes as usize;
			if header.tag == ChunkHeader::DATA_TAG {
				break size;
			}
			data = &rest[size..];
		};

		MmapedRiffPcm {
			samples_per_second: bytes_per_sec / num_channels as u32 / size_of::<i16>() as u32,
			channels,
			samples: cast_slice::<u8, i16>(&rest[..size]),
		}
	}
}
