use core::error::Error;
use core::fmt;
use std::io::{self, SeekFrom};

use bytemuck::{
	Pod,
	Zeroable,
	must_cast_mut,
	must_cast_ref,
	must_cast_slice_mut,
	zeroed_slice_box,
};

// TODO newtype this, repr transparent, pod, zeroable, to/from byte?
type Sample = i16;

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct ChunkHeader {
	tag: [u8; 4],
	len: u32,
}
impl ChunkHeader {
	const DATA_TAG: [u8; 4] = *b"data";
	const FORMAT_TAG: [u8; 4] = *b"fmt ";
	const RIFF_TAG: [u8; 4] = *b"RIFF";
}

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
		<Self as fmt::Display>::fmt(&self, f)
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

#[repr(u16)]
enum DataFormat {
	Pcm = 0x0001,
}
impl DataFormat {
	const EXTENSIBLE: u16 = 0xfffe;
	const PCM: u16 = 0x0001;

	const fn new(format: u16) -> Result<Self, FormatSizeConvError> {
		match format {
			Self::PCM => Ok(Self::Pcm),
			_size => todo!(),
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

	fn is_valid(&self) -> bool {
		// self.riff_header() == RIFF_TAG ...  && true
		todo!()
	}
	// TODO: fn to extract Result<enumized data_format>

	// TODO: fn validate(&self) -> bool or maybe Result<(), Error impl Into io::Error>
}

#[non_exhaustive]
enum WaveExtValidError {
	IncorrectSize(u16),
	IncorrectTag,
}
impl fmt::Display for WaveExtValidError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::IncorrectSize(size) => {
				// TODO
				write!(
					f,
					"format chunk size of {size} is invalid. must be one of 16, 18, or 40",
				)
			},
			Self::IncorrectTag => todo!(),
		}
	}
}
impl fmt::Debug for WaveExtValidError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<Self as fmt::Display>::fmt(&self, f)
	}
}
impl Error for WaveExtValidError {}

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

	fn data_format(&self) -> Result<DataFormat, WaveExtValidError> {
		if self.extension_size != Self::EXTENSION_SIZE {
			return Err(WaveExtValidError::IncorrectSize(self.extension_size));
		}
		if self.extention_tag != Self::TAG {
			println!("TAG {:X?}", self.extention_tag);
			return Err(WaveExtValidError::IncorrectTag);
		}
		// TODO check if data format is actually pcm
		Ok(DataFormat::Pcm)
	}
}

pub fn parse1(b: &[u8]) {
	let mut arr = [0u32; size_of::<RawRiffData>() / 4];
	must_cast_slice_mut::<_, u8>(&mut arr).copy_from_slice(&b[..size_of::<RawRiffData>()]);

	let riff: &RawRiffData = must_cast_ref(&arr);

	println!("{riff:#?}");
}

pub struct RiffWavePcm {
	pub samples_per_second: u32,
	pub samples: Box<[i16]>,
}
impl RiffWavePcm {
	fn parse_from_data_chunk(
		source: impl io::Read + io::Seek,
		samples: usize,
		channels: usize,
	) -> io::Result<Self> {
		todo!()
	}
}

pub fn parse(source: impl io::Read + io::Seek) -> io::Result<RiffWavePcm> {
	/// https://github.com/ascent12/average/blob/master/avg.c
	fn avg_perfect(slice: &[i16]) -> i16 {
		let n = slice.len() as i64;
		let mut avg = 0i16;

		let mut error = 0;

		for i in 0..slice.len() {
			error += (slice[i] as i64 % n) as i16;
			avg += ((slice[i] as i64 / n) + (error as i64 / n)) as i16;
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
	let mut riff_data = RawRiffData::zeroed();

	source.read_exact(must_cast_mut::<_, [_; 36]>(&mut riff_data))?;

	if riff_data.riff_header.tag != ChunkHeader::RIFF_TAG {
		return Err(io::Error::other("chunk header must be \"RIFF\""));
	}

	if riff_data.format_header.tag != ChunkHeader::FORMAT_TAG {
		return Err(io::Error::other("format tag must be \"fmt \""));
	}

	let format_size = FormatSize::new(riff_data.format_header.len).map_err(io::Error::other)?;

	// TODO: this is in need of abstraction.
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
				return Err(io::Error::other("extension size is not zero"));
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
			if ext.data_format != DataFormat::PCM.to_le_bytes() {
				if riff_data.data_format != DataFormat::PCM {
					return Err(io::Error::other("data format must be the PCM data format"));
				}
			}
		},
	}

	// fn skip_until_data_chunk(source) -> ChunkHeader
	let num_bytes = loop {
		let mut header = ChunkHeader::zeroed();
		source.read_exact(must_cast_mut::<_, [_; 8]>(&mut header))?;

		if header.tag == ChunkHeader::DATA_TAG {
			break header.len as usize;
		}

		source.seek(SeekFrom::Current(header.len as i64))?;
	};

	// TODO: replace with size_of::<Sample>();
	let num_samples = num_bytes / size_of::<i16>();
	let num_channels = riff_data.num_channels as usize;

	let num_samples_per_channel = num_samples / num_channels;

	let mut buf = zeroed_slice_box::<i16>(num_samples_per_channel);

	if num_channels == 1 {
		source.read_exact(must_cast_slice_mut::<_, u8>(&mut buf))?;
	} else {
		let mut index = 0;
		let mut num_remaining_samples = num_samples;
		let mut temp_samples = zeroed_slice_box::<i16>(num_samples.min(10_000_000 * num_channels));

		while num_remaining_samples > 0 {
			let num_samples_to_read = num_remaining_samples.min(temp_samples.len());
			let temp_samples_buf = &mut temp_samples[..num_samples_to_read];
			source.read_exact(must_cast_slice_mut::<_, u8>(temp_samples_buf))?;
			num_remaining_samples -= num_samples_to_read;

			// TODO make this zip an iterator of buf[slice..slice].iter_mut() and the chunks_exact
			for chunk in temp_samples_buf.chunks_exact(num_channels) {
				buf[index] = avg_perfect(chunk);
				index += 1;
			}
		}
	}

	Ok(RiffWavePcm {
		samples_per_second: riff_data.bytes_per_second
			/ size_of::<i16>() as u32
			/ riff_data.num_channels as u32,
		samples: buf,
	})
}
