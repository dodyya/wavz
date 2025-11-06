// use std::io::{Read, Result, Seek};

use std::io::{self, SeekFrom};
use std::ptr::{NonNull, slice_from_raw_parts_mut};

#[repr(C, packed(1))]
struct ChunkHeader {
	tag: [u8; 4],
	len: [u8; 4],
}
impl ChunkHeader {
	const fn zeroed() -> Self {
		Self { tag: [0; 4], len: [0; 4] }
	}

	const fn as_bytes_mut(&mut self) -> &mut [u8; size_of::<Self>()] {
		// TODO: SAFETY comment
		let mut ptr = NonNull::from_mut(self).cast();
		unsafe { ptr.as_mut() }
	}

	fn remove_later_print(&self) {
		println!(
			"tag: {:?}, len: {}",
			str::from_utf8(&self.tag),
			u32::from_le_bytes(self.len)
		);
	}
}

// TODO: consistent naming on tag vs id
// TODO: impl RiffData {safe fn [u8; N]->Self}
// TODO: consider using actual u32 fields for the lengths and making this packed(4) instead
#[repr(C, packed(1))]
struct RiffData {
	riff_id: [u8; 4],
	riff_size: [u8; 4],
	wave_id: [u8; 4],
	format_id: [u8; 4],
	format_size: [u8; 4],
	format_tag: [u8; 2],
	num_channels: [u8; 2],
	sample_blocks_per_sec: [u8; 4],
	bytes_per_second: [u8; 4],
	sample_block_size: [u8; 2],
	bits_per_sample: [u8; 2],
}
impl RiffData {
	const RIFF_TAG: [u8; 4] = *b"RIFF";

	// TODO: more constants

	// TODO: fn validate(&self) -> bool

	const fn zeroed() -> Self {
		Self {
			riff_id: [0; 4],
			riff_size: [0; 4],
			wave_id: [0; 4],
			format_id: [0; 4],
			format_size: [0; 4],
			format_tag: [0; 2],
			num_channels: [0; 2],
			sample_blocks_per_sec: [0; 4],
			bytes_per_second: [0; 4],
			sample_block_size: [0; 2],
			bits_per_sample: [0; 2],
		}
	}

	const fn as_bytes_mut(&mut self) -> &mut [u8; size_of::<Self>()] {
		// TODO: SAFETY comment
		let mut ptr = NonNull::from_mut(self).cast();
		unsafe { ptr.as_mut() }
	}
}

#[repr(C, packed(1))]
struct ExtentionSize {
	extension_size: [u8; 2], // 0 or 22
}
impl ExtentionSize {
	const fn zeroed() -> Self {
		Self { extension_size: [0; 2] }
	}

	const fn as_bytes_mut(&mut self) -> &mut [u8; size_of::<Self>()] {
		let mut ptr = NonNull::from_mut(self).cast();
		unsafe { ptr.as_mut() }
	}
}

#[repr(C, packed(1))]
struct WaveExtention {
	extension_size: ExtentionSize,
	valid_bits_per_sample: [u8; 2],
	speaker_position_mask: [u8; 4],
	format_id: [u8; 2],
	fixed_string: [u8; 14],
}
impl WaveExtention {
	const fn zeroed() -> Self {
		Self {
			extension_size: ExtentionSize::zeroed(),
			valid_bits_per_sample: [0; 2],
			speaker_position_mask: [0; 4],
			format_id: [0; 2],
			fixed_string: [0; 14],
		}
	}

	const fn as_bytes_mut(&mut self) -> &mut [u8; size_of::<Self>()] {
		let mut ptr = NonNull::from_mut(self).cast();
		unsafe { ptr.as_mut() }
	}
}

// TODO should not take &[u8], read into chunk_size buffers on the heap?
pub fn parse1(b: &[u8]) {
	let sliceref: &[u8; size_of::<RiffData>()] =
		<_>::try_from(&b[..size_of::<RiffData>()]).unwrap();
	let riff = unsafe { core::mem::transmute::<_, &RiffData>(sliceref) };

	println!("{:?}", str::from_utf8(&riff.riff_id).unwrap());
	println!("{:?}", u32::from_le_bytes(riff.riff_size));
	println!("{:?}", str::from_utf8(&riff.wave_id).unwrap());
	println!("{:?}", str::from_utf8(&riff.format_id).unwrap());
	println!("{:?}", u32::from_le_bytes(riff.format_size));
	println!("{:#X}", u16::from_le_bytes(riff.format_tag));
	println!("{:?}", u16::from_le_bytes(riff.num_channels));
	println!("{:?}", u32::from_le_bytes(riff.sample_blocks_per_sec));
	println!("{:?}", u32::from_le_bytes(riff.bytes_per_second));
	println!("{:?}", u16::from_le_bytes(riff.sample_block_size));
	println!("{:?}", u16::from_le_bytes(riff.bits_per_sample));

	// println!("{:#b}", u32::from_le_bytes(riff.dwChannelMask));
	// println!("{:X?}", riff.SubFormat);
}

// pub fn parse1(b: &[u8]) {
// 	let mut slice = b;

// 	println!("{:?}", str::from_utf8(&slice[0..4]));
// 	let len = u32::from_le_bytes(<_>::try_from(&slice[4..8]).unwrap());
// 	println!("{len}");
// 	println!("{:?}", str::from_utf8(&slice[8..12]));
// 	slice = &slice[12..];

// 	while !slice.is_empty() {
// 		let string = str::from_utf8(&slice[0..4]);
// 		println!("{:?}", string);
// 		let len = u32::from_le_bytes(<_>::try_from(&slice[4..8]).unwrap());
// 		println!("{len}");
// 		let len = len as usize;

// 		if string == Ok("LIST") {
// 			let string2 = String::from_utf8_lossy(&slice[..8 + len]);
// 			println!("{string2:?}");
// 		}

// 		slice = &slice[8 + len..];
// 	}
// }

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
	//      but then that means i have to use the shitty unstable api for it
	let mut riff_data = RiffData::zeroed();

	source.read_exact(riff_data.as_bytes_mut())?;

	// TODO: make id constants on RiffData, check constants against fields
	// or just make a check function on RiffData itself

	let format_id = match u32::from_le_bytes(riff_data.format_size) {
		16 => riff_data.format_tag,
		18 => {
			source.read_exact(ExtentionSize::zeroed().as_bytes_mut())?;
			// could assert that these are zero, but we can be lenient
			riff_data.format_tag
		},
		40 => {
			let mut extention = WaveExtention::zeroed();
			source.read_exact(extention.as_bytes_mut())?;

			extention.format_id
		},
		_ => {
			return Err(io::Error::other(
				"invalid format_size length: must be 16, 18, or 40",
			));
		},
	};

	let file_len = size_of::<ChunkHeader>() as u32 + u32::from_le_bytes(riff_data.riff_size);
	let mut chunk_header = ChunkHeader::zeroed();
	let mut cur_pos = 0;

	source.read_exact(chunk_header.as_bytes_mut())?;

	while cur_pos < file_len {
		chunk_header.remove_later_print(); // TODO remove
		if chunk_header.tag == *b"data" {
			break;
		}

		// TODO: replace try_into().unwrap() with something better
		cur_pos = source
			.seek(SeekFrom::Current(
				u32::from_le_bytes(chunk_header.len) as i64
			))?
			.try_into()
			.unwrap();

		source.read_exact(chunk_header.as_bytes_mut())?;
	}
	if cur_pos >= file_len {
		return Err(io::Error::other("no data section found"));
	}

	let num_channels = u16::from_le_bytes(riff_data.num_channels) as usize;
	let num_samples_per_channel =
		u32::from_le_bytes(chunk_header.len) as usize / size_of::<i16>() / num_channels;

	dbg!(num_channels, num_samples_per_channel);

	fn make_sample_buf(samples_per_channel: usize) -> NonNull<[i16]> {
		let buf = Box::<[i16]>::new_zeroed_slice(samples_per_channel);
		let ptr = Box::into_raw(buf) as *mut [i16];
		NonNull::new(ptr).unwrap()
	}

	let buf = make_sample_buf(num_samples_per_channel);

	let mut samples_ptr: NonNull<[u8]> = NonNull::slice_from_raw_parts(
		buf.cast(),
		num_samples_per_channel * size_of::<i16>() / size_of::<u8>(),
	);

	// switch from passing around box slice to passing around &mut slice
	// then make NN::s_f_r_p.as_mut helper to transmute when needed
	{
		// SAFETY: original box has been destructed, length has been scaled, u8 and i16 are POD,
		// u8 align <= i16 align
		let samples_slice = unsafe { samples_ptr.as_mut() };

		dbg!(samples_slice.len());
		samples_slice.as_mut_ptr();

		source.read_exact(samples_slice)?;

		if num_channels > 1 {
			for i in 0..samples_slice.len() / 4 {
				// avg_perfect(&samples_slice[i * 4..i * 4 + num_channels]);
			}

			// average original slice
			// TODO: allocate identical slice, fill, average, append to original slice
		}
	}

	let samples_buf = unsafe { Box::<[i16]>::from_raw(buf.as_ptr()) };

	// dbg!(data_size);
	// let buf = {
	// 	let ptr = Box::into_raw(Box::<[i16]>::new_zeroed_slice(data_size / 2)) as *mut i16;
	// 	// *2 because size(i16)/size(u8)
	// 	let slice = unsafe { &mut *slice_from_raw_parts_mut(ptr.cast::<u8>(), data_size) };
	// 	dbg!(slice.len());

	// 	source.read_exact(slice)?;
	// 	unsafe {
	// 		Box::from_raw(slice_from_raw_parts_mut(
	// 			slice.as_mut_ptr().cast::<i16>(),
	// 			slice.len() / 2,
	// 		))
	// 	}
	// };
	// dbg!(buf.len());

	// TODO: validate `format_data`, error if bad (not WAVE_FORMAT_PCM, that is all i cba to support rn)
	// TODO: we have all the data we need from header. keep going, parse next blocks
	// if blocks != data, Seek over them
	// if block == data, read length, allocate buffer, write to buffer, construct return type struct, exit early.
	// else, no data found, error

	Ok(RiffWavePcm {
		samples_per_second: u32::from_le_bytes(riff_data.sample_blocks_per_sec)
			/ u16::from_le_bytes(riff_data.num_channels) as u32,
		samples: samples_buf,
	})
}
