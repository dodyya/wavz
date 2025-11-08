use std::error::Error;
use std::fmt::{self, Display};
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

	fn is_data(self) -> bool {
		self.tag == Self::DATA_TAG
	}

	fn is_format(self) -> bool {
		self.tag == Self::FORMAT_TAG
	}

	fn is_riff(self) -> bool {
		self.tag == Self::RIFF_TAG
	}
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
	Extensible = 0xfffe,
}

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct RawRiffData {
	riff_header: ChunkHeader,
	wave_tag: [u8; 4],
	format_header: ChunkHeader,
	data_format: [u8; 2],
	num_channels: u16,
	sample_blocks_per_sec: u32,
	bytes_per_second: u32,
	sample_block_size: u16,
	bits_per_sample: u16,
}
impl RawRiffData {
	const WAVE_TAG: [u8; 4] = *b"WAVE";

	fn is_valid(&self) -> bool {
		self.riff_header.is_riff() && true
	}
	// TODO: fn to extract Result<enumized data_format>

	// TODO: fn validate(&self) -> bool or maybe Result<(), Error impl Into io::Error>
}

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(transparent)]
struct EmptyExtention(u16);
impl EmptyExtention {
	const EXTENTION_SIZE: u16 = 0;

	fn is_valid(self) -> bool {
		self.0 == Self::EXTENTION_SIZE
	}
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
	// TODO: constants for validation, like the tag + size + data_format

	// TODO: fn validate(&self) -> bool
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
	// TODO could be uninit instead of zero, bench later to see if it matters
	let mut riff_data = RawRiffData::zeroed();

	source.read_exact(must_cast_mut::<_, [_; 36]>(&mut riff_data))?;

	if !riff_data.riff_header.is_riff() {
		return Err(io::Error::other("chunk header must be \"RIFF\""));
	}

	// TODO validate read here when implemented

	let format_size = FormatSize::new(riff_data.format_header.len).map_err(io::Error::other)?;

	let data_format = match format_size {
		FormatSize::Size16 => riff_data.data_format,
		FormatSize::Size18 => {
			let mut ext = EmptyExtention::zeroed();
			source.read_exact(must_cast_mut::<_, [_; 2]>(&mut ext))?;

			if !ext.is_valid() {
				return Err(io::Error::other(format!(
					"incorrect extention length of {}, expected 0",
					ext.0
				)));
			}

			riff_data.data_format
		},
		FormatSize::Size40 => {
			let mut extention = WaveExtention::zeroed();
			source.read_exact(must_cast_mut::<_, [_; 24]>(&mut extention))?;
			// TODO: assert that extsize=22, that data is correct when implemented

			extention.data_format
		},
	};

	// TODO turn into ext method or standalone, make return instead of panic
	// actually use it as an enum, right? then make extmethod on enum or use matches! or something
	assert!(data_format == [1, 0]); // PCM

	let num_bytes = loop {
		let mut header = ChunkHeader::zeroed();
		source.read_exact(must_cast_mut::<_, [_; 8]>(&mut header))?;

		if header.is_data() {
			break header.len as usize;
		}

		source.seek(SeekFrom::Current(header.len as i64))?;
	};

	let num_samples = num_bytes / size_of::<i16>();
	let num_channels = riff_data.num_channels as usize;

	let num_samples_per_channel = num_samples / num_channels;

	// if block == data, read length, allocate buffer, write to buffer, construct return type struct, exit early.

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

			// TODO this is terrible, fix
			for (idx, chunk) in temp_samples_buf.chunks_exact(num_channels).enumerate() {
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
