//! Port of src/smw/network/FileCompressor.cpp

#[cfg(not(target_os = "emscripten"))]
use libz_sys::{compress, compress2, compressBound, uLong, uncompress, Z_BUF_ERROR, Z_OK};
#[cfg(target_os = "emscripten")]
use emscripten_zlib::{compress, compress2, compressBound, uLong, uncompress, Z_BUF_ERROR, Z_OK};
use std::io::{Read, Seek, SeekFrom, Write};

const COMPRESSION_SIZE_LIMIT: u64 = 20000;

/// Emscripten's zlib port (`-sUSE_ZLIB=1`), as upstream links: libz-sys builds zlib for wasm32 with
/// `Z_SOLO`, where `compress` and `uncompress` have no allocator and always fail.
#[cfg(target_os = "emscripten")]
mod emscripten_zlib {
    pub type uLong = std::os::raw::c_ulong;
    pub const Z_OK: i32 = 0;
    pub const Z_BUF_ERROR: i32 = -5;
    extern "C" {
        pub fn compress(dest: *mut u8, dest_len: *mut uLong, source: *const u8, source_len: uLong) -> i32;
        pub fn compress2(dest: *mut u8, dest_len: *mut uLong, source: *const u8, source_len: uLong, level: i32) -> i32;
        pub fn compressBound(source_len: uLong) -> uLong;
        pub fn uncompress(dest: *mut u8, dest_len: *mut uLong, source: *const u8, source_len: uLong) -> i32;
    }
}

/// `CompressedData`: an owned buffer; empty means invalid.
pub struct CompressedData {
    pub data: Vec<u8>,
}

impl CompressedData {
    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn is_valid(&self) -> bool {
        !self.data.is_empty()
    }
}

pub struct FileCompressor;

impl FileCompressor {
    /// Buffer layout: `output_header_offset` zero bytes, uncompressed size (u16), compressed size (u16), zlib data.
    pub fn compress(input_path: &str, output_header_offset: usize) -> CompressedData {
        let result = (|| -> Option<Vec<u8>> {
            if input_path.len() < 5 {
                return None;
            }

            let Ok(mut input_file) = std::fs::File::open(input_path) else {
                println!("[error] Could not open {}", input_path);
                return None;
            };

            let input_size = input_file.seek(SeekFrom::End(0)).ok()?;
            input_file.seek(SeekFrom::Start(0)).ok()?;

            if input_size > COMPRESSION_SIZE_LIMIT || input_size < 10 {
                println!("[error] File {} has irregular size", input_path);
                return None;
            }

            let mut input_buffer = vec![0u8; input_size as usize];
            if input_file.read_exact(&mut input_buffer).is_err() {
                println!("[error] File reading error ({})", input_path);
                return None;
            }
            drop(input_file);

            unsafe {
                let mut compressed_size_ulong: uLong = compressBound(input_size as uLong);
                if compressed_size_ulong as u64 > COMPRESSION_SIZE_LIMIT {
                    println!("[error] File is too big: {}", input_path);
                    return None;
                }

                let mut compressed_buffer = vec![0u8; output_header_offset + 4 + compressed_size_ulong as usize];

                let return_value = compress(
                    compressed_buffer.as_mut_ptr().add(output_header_offset + 4),
                    &mut compressed_size_ulong,
                    input_buffer.as_ptr(),
                    input_size as uLong,
                );
                if return_value != Z_OK {
                    println!("[error] Out of memory, could not compress {}", input_path);
                    return None;
                }

                let stored_full_size = input_size as u16;
                let stored_compressed_size = compressed_size_ulong as u16;
                compressed_buffer[output_header_offset..output_header_offset + 2].copy_from_slice(&stored_full_size.to_ne_bytes());
                compressed_buffer[output_header_offset + 2..output_header_offset + 4].copy_from_slice(&stored_compressed_size.to_ne_bytes());

                compressed_buffer.truncate(compressed_size_ulong as usize + output_header_offset + 4);
                Some(compressed_buffer)
            }
        })();

        match result {
            Some(data) => CompressedData { data },
            None => {
                println!("[error] Compression failed");
                CompressedData { data: Vec::new() }
            }
        }
    }

    /// Decompresses a buffer in the layout `compress` writes (starting at the size fields) into `output_path`.
    pub fn decompress(input_buffer: &[u8], output_path: &str) -> bool {
        let Ok(mut output_file) = std::fs::File::create(output_path) else {
            print!("[error] Could not create {}", output_path);
            return false;
        };

        let ok = (|| -> bool {
            if input_buffer.len() < 4 {
                return false;
            }
            let stored_full_size = u16::from_ne_bytes([input_buffer[0], input_buffer[1]]);
            let stored_compressed_size = u16::from_ne_bytes([input_buffer[2], input_buffer[3]]);

            if stored_full_size as u64 > COMPRESSION_SIZE_LIMIT || stored_compressed_size as u64 > COMPRESSION_SIZE_LIMIT {
                println!("[error] Irregular size stored, possibly corrupted?");
                return false;
            }
            if input_buffer.len() < 4 + stored_compressed_size as usize {
                println!("[error] Out of memory, could not decompress");
                return false;
            }

            let mut output_size_ulong: uLong = stored_full_size as uLong;
            let mut output_buffer = vec![0u8; stored_full_size as usize];

            let return_value = unsafe {
                uncompress(output_buffer.as_mut_ptr(), &mut output_size_ulong, input_buffer.as_ptr().add(4), stored_compressed_size as uLong)
            };
            if return_value != Z_OK {
                println!("[error] Out of memory, could not decompress");
                return false;
            }

            if output_file.write_all(&output_buffer[..output_size_ulong as usize]).is_err() || stored_full_size as uLong != output_size_ulong {
                println!("[error] File writing error ({})", output_path);
                return false;
            }
            true
        })();

        if !ok {
            println!("[error] Decompression failed");
        }
        ok
    }
}

/// Not in the C++: a zlib stream at the best compression, for replay checkpoints (smw/harness.rs).
pub fn deflate(data: &[u8]) -> Vec<u8> {
    unsafe {
        let mut len = compressBound(data.len() as uLong);
        let mut out = vec![0u8; len as usize];
        let rc = compress2(out.as_mut_ptr(), &mut len, data.as_ptr(), data.len() as uLong, 9);
        assert_eq!(rc, Z_OK, "zlib compress2 failed");
        out.truncate(len as usize);
        out
    }
}

/// Not in the C++: the data of a zlib stream, or None when it is damaged.
pub fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    let mut cap = data.len().max(256) * 8;
    loop {
        let mut out = vec![0u8; cap];
        let mut len = cap as uLong;
        match unsafe { uncompress(out.as_mut_ptr(), &mut len, data.as_ptr(), data.len() as uLong) } {
            Z_OK => {
                out.truncate(len as usize);
                return Some(out);
            }
            Z_BUF_ERROR if cap < 1 << 26 => cap *= 2,
            _ => return None,
        }
    }
}

/// zlib's stream API, declared here because Emscripten's zlib has no Rust binding.
mod zstream {
    use std::os::raw::{c_char, c_int, c_uint, c_ulong, c_void};

    pub const Z_OK: c_int = 0;
    pub const Z_STREAM_END: c_int = 1;
    pub const Z_NO_FLUSH: c_int = 0;

    #[repr(C)]
    pub struct ZStream {
        pub next_in: *const u8,
        pub avail_in: c_uint,
        pub total_in: c_ulong,
        pub next_out: *mut u8,
        pub avail_out: c_uint,
        pub total_out: c_ulong,
        pub msg: *const c_char,
        pub state: *mut c_void,
        pub zalloc: *mut c_void,
        pub zfree: *mut c_void,
        pub opaque: *mut c_void,
        pub data_type: c_int,
        pub adler: c_ulong,
        pub reserved: c_ulong,
    }

    extern "C" {
        pub fn zlibVersion() -> *const c_char;
        pub fn inflateInit2_(strm: *mut ZStream, window_bits: c_int, version: *const c_char, stream_size: c_int) -> c_int;
        #[link_name = "inflate"]
        pub fn inflate_stream(strm: *mut ZStream, flush: c_int) -> c_int;
        pub fn inflateReset(strm: *mut ZStream) -> c_int;
        pub fn inflateEnd(strm: *mut ZStream) -> c_int;
    }
}

/// Not in the C++: the data of a gzip file (a .smwrp recording), or None when it is damaged.
pub fn gunzip(data: &[u8]) -> Option<Vec<u8>> {
    use std::os::raw::{c_int, c_uint};
    use zstream::*;
    let mut out = Vec::with_capacity(data.len() * 8);
    let mut buf = vec![0u8; 1 << 16];
    unsafe {
        let mut s: ZStream = std::mem::zeroed();
        if inflateInit2_(&mut s, 15 + 16, zlibVersion(), std::mem::size_of::<ZStream>() as c_int) != Z_OK {
            return None;
        }
        s.next_in = data.as_ptr();
        s.avail_in = data.len() as c_uint;
        let rc = loop {
            s.next_out = buf.as_mut_ptr();
            s.avail_out = buf.len() as c_uint;
            let rc = inflate_stream(&mut s, Z_NO_FLUSH);
            out.extend_from_slice(&buf[..buf.len() - s.avail_out as usize]);
            match rc {
                Z_STREAM_END if s.avail_in > 0 => {
                    inflateReset(&mut s);
                }
                Z_OK => {}
                _ => break rc,
            }
        };
        inflateEnd(&mut s);
        (rc == Z_STREAM_END).then_some(out)
    }
}

/// Not in the C++: a file's bytes, gunzipped when they start with the gzip magic.
pub fn read_maybe_gzip(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<u8>> {
    let bytes = std::fs::read(path)?;
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Ok(bytes);
    }
    gunzip(&bytes).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "damaged gzip"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_a_map() {
        let map = concat!(env!("CARGO_MANIFEST_DIR"), "/data/maps/0smw.map");
        let c = FileCompressor::compress(map, 3);
        assert!(c.is_valid());
        let out = std::env::temp_dir().join(format!("smw_fc_{}.map", std::process::id()));
        assert!(FileCompressor::decompress(&c.data[3..], out.to_str().unwrap()));
        assert_eq!(std::fs::read(map).unwrap(), std::fs::read(&out).unwrap());
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn deflate_round_trips() {
        let data: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8 ^ (i / 997) as u8).collect();
        let z = deflate(&data);
        assert!(z.len() < data.len() / 4);
        assert_eq!(inflate(&z).unwrap(), data);
        assert_eq!(inflate(&deflate(b"")).unwrap(), b"");
        assert!(inflate(&z[..z.len() / 2]).is_none());
        assert!(inflate(b"not zlib").is_none());
    }

    #[test]
    fn gunzip_reads_gzip_members() {
        let one: &[u8] = &[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0x53, 0x76, 0x50, 0x28, 0x4e, 0x4d, 0x4d, 0xb1, 0x35, 0xe7, 0x32, 0x34, 0x50, 0xc8, 0x4e, 0xad, 0x54, 0x48, 0xe4, 0x02, 0x00, 0x21, 0x8c, 0xb0, 0xe4, 0x13, 0x00, 0x00, 0x00];
        let two: &[u8] = &[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0x2b, 0x49, 0xcc, 0xcc, 0xe1, 0x02, 0x00, 0x6e, 0x1c, 0x71, 0x27, 0x05, 0x00, 0x00, 0x00];
        assert_eq!(gunzip(one).unwrap(), b"#@ seed=7\n10 key a\n");
        assert_eq!(gunzip(&[one, two].concat()).unwrap(), b"#@ seed=7\n10 key a\ntail\n");
        assert!(gunzip(&one[..one.len() - 3]).is_none());
        assert!(gunzip(&deflate(b"zlib, not gzip")).is_none());
        let big: Vec<u8> = (0..300_000u32).map(|i| b"0123456789 key\n"[(i % 15) as usize]).collect();
        let path = std::env::temp_dir().join(format!("smw_gz_{}.smwrp", std::process::id()));
        std::fs::write(&path, &big).unwrap();
        assert_eq!(read_maybe_gzip(&path).unwrap(), big);
        std::fs::write(&path, [0x1f, 0x8b, 0, 0]).unwrap();
        assert!(read_maybe_gzip(&path).is_err());
        let _ = std::fs::remove_file(path);
    }
}
