use std::io::{self, SeekFrom};
use std::ptr::NonNull;

use bytemuck::{Pod, Zeroable, must_cast_mut, must_cast_ref, must_cast_slice_mut, zeroed};

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct ChunkHeader {
	tag: [u8; 4],
	len: u32,
}

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct RiffData {
	riff_header: ChunkHeader,
	wave_tag: [u8; 4],
	format_tag: [u8; 4],
	format_size: u32,
	data_format: [u8; 2],
	num_channels: u16,
	sample_blocks_per_sec: u32,
	bytes_per_second: u32,
	sample_block_size: u16,
	bits_per_sample: u16,
}
impl RiffData {
	const RIFF_TAG: [u8; 4] = *b"RIFF";

	// TODO: more constants for validation. ex: *b"dat "
	// TODO: fn to extract Result<enumized data_format>

	// TODO: fn validate(&self) -> bool or maybe Result<(), Error impl Into io::Error>
}

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct EmptyExtention {
	extension_size: u16, // 0
}
impl EmptyExtention {
	// TODO: constants for validation. 0
	// TODO: fn validate(&self) -> bool
}

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct WaveExtention {
	extension_size: u16,
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
	let mut arr = [0u32; size_of::<RiffData>() / 4];
	must_cast_slice_mut::<_, u8>(&mut arr).copy_from_slice(&b[..size_of::<RiffData>()]);

	let riff: &RiffData = must_cast_ref(&arr);

	println!("{riff:#?}");
}

pub struct RiffWavePcm {
	pub samples_per_second: u32,
	pub samples: Box<[i16]>,
}

pub fn parse(source: impl io::Read + io::Seek) -> io::Result<RiffWavePcm> {
	// fn avg_perfect(slice: &[i16]) -> i16 {
	// 	// https://github.com/ascent12/average/blob/master/avg.c

	// 	let n = slice.len() as i64;
	// 	let mut avg = 0i16;

	// 	let mut error = 0;

	// 	for i in 0..slice.len() {
	// 		error += (slice[i] as i64 % n) as i16;
	// 		avg += ((slice[i] as i64 / n) + (error as i64 / n)) as i16;
	// 		error = (error as i64 % n) as i16;
	// 	}

	// 	if avg < 0 && error > 0 {
	// 		avg += 1;
	// 	} else if avg > 0 && error < 0 {
	// 		avg -= 1;
	// 	}

	// 	avg
	// }

	let mut source = source;
	// TODO could be uninit instead of zero, bench later to see if it matters
	let mut riff_data = RiffData::zeroed();

	source.read_exact(must_cast_mut::<_, [_; 36]>(&mut riff_data))?;

	// TODO validate read here when implemented

	let data_format = match riff_data.format_size {
		16 => riff_data.data_format,
		18 => {
			source.read_exact(must_cast_mut::<_, [_; 2]>(&mut EmptyExtention::zeroed()))?;
			// TODO: assert that this read is zero when implemented

			riff_data.data_format
		},
		40 => {
			let mut extention = WaveExtention::zeroed();
			source.read_exact(must_cast_mut::<_, [_; 24]>(&mut extention))?;
			// TODO: assert that extsize=22, that data is correct when implemented

			extention.data_format
		},
		_ => {
			// TODO parse, don't validate: make this returned by ? on parse fn
			return Err(io::Error::other(
				"invalid format_size length: must be 16, 18, or 40",
			));
		},
	};

	let file_len = size_of::<ChunkHeader>() as u32 + riff_data.riff_header.len;
	let mut chunk_header = ChunkHeader::zeroed();
	let mut cur_pos = 0;

	source.read_exact(must_cast_mut::<_, [_; 8]>(&mut chunk_header))?;

	while cur_pos < file_len {
		if chunk_header.tag == *b"data" {
			break;
		}

		// TODO: replace try_into().unwrap() with something better
		cur_pos = source
			.seek(SeekFrom::Current(chunk_header.len as i64))?
			.try_into()
			.unwrap();

		source.read_exact(must_cast_mut::<_, [_; 8]>(&mut chunk_header))?;
	}
	if cur_pos >= file_len {
		return Err(io::Error::other("no data section found"));
	}

	let num_channels = riff_data.num_channels as usize;
	let num_samples_per_channel = chunk_header.len as usize / size_of::<i16>() / num_channels;

	dbg!(num_channels, num_samples_per_channel);

	// TODO: we have all the data we need from header. keep going, parse next blocks
	// if blocks != data, Seek over them
	// if block == data, read length, allocate buffer, write to buffer, construct return type struct, exit early.
	// else, no data found, error

	Ok(RiffWavePcm {
		samples_per_second: riff_data.sample_blocks_per_sec / riff_data.num_channels as u32,
		samples: todo!(),
	})
}
