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
	fn is_data(&self) -> bool {
		self.tag == *b"data"
	}
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
	fn avg_perfect(slice: &[i16]) -> i16 {
		// https://github.com/ascent12/average/blob/master/avg.c

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

	dbg!(num_samples, num_channels, num_samples_per_channel);

	// if block == data, read length, allocate buffer, write to buffer, construct return type struct, exit early.

	let mut buf = zeroed_slice_box::<i16>(num_samples_per_channel);
	println!("T allocated buf size {num_samples_per_channel}");

	if num_channels == 1 {
		source.read_exact(must_cast_slice_mut::<_, u8>(&mut buf))?;
	} else {
		let mut index = 0;
		let mut num_remaining_samples = num_samples;
		let mut temp_samples = zeroed_slice_box::<i16>(num_samples.min(10_000_000 * num_channels));
		println!("T allocated temp buf size {}", temp_samples.len());

		while num_remaining_samples > 0 {
			let num_samples_to_read = num_remaining_samples.min(temp_samples.len());
			let temp_samples_buf = &mut temp_samples[..num_samples_to_read];
			println!("T reading {num_samples_to_read} samples");
			source.read_exact(must_cast_slice_mut::<_, u8>(temp_samples_buf))?;
			num_remaining_samples -= num_samples_to_read;

			// compress
			for (idx, chunk) in temp_samples_buf.chunks_exact(num_channels).enumerate() {
				buf[index] = avg_perfect(chunk);
				index += 1;
			}
		}
		dbg!(index);
	}

	Ok(RiffWavePcm {
		samples_per_second: riff_data.bytes_per_second
			/ size_of::<i16>() as u32
			/ riff_data.num_channels as u32,
		samples: buf,
	})
}
