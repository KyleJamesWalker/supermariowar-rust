//! Port of src/smw/network/FileCompressor.cpp

use flate2::read::{MultiGzDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Seek, SeekFrom, Write};

const COMPRESSION_SIZE_LIMIT: u64 = 20000;

/// zlib's `compressBound`.
fn compress_bound(source_len: u64) -> u64 {
    source_len + (source_len >> 12) + (source_len >> 14) + (source_len >> 25) + 13
}

/// zlib's `compress2`: a zlib stream of `data`.
fn zlib_compress(data: &[u8], level: u32) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).expect("writing to a Vec");
    e.finish().expect("writing to a Vec")
}

/// The data of the zlib stream at the start of `data`, or None when it is damaged or longer than `limit`.
fn zlib_decompress(data: &[u8], limit: u64) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let read = ZlibDecoder::new(data).take(limit + 1).read_to_end(&mut out);
    (read.is_ok() && out.len() as u64 <= limit).then_some(out)
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

            if compress_bound(input_size) > COMPRESSION_SIZE_LIMIT {
                println!("[error] File is too big: {}", input_path);
                return None;
            }

            let compressed = zlib_compress(&input_buffer, 6);
            if compressed.len() as u64 > compress_bound(input_size) {
                println!("[error] Out of memory, could not compress {}", input_path);
                return None;
            }
            let mut compressed_buffer = vec![0u8; output_header_offset + 4];
            let stored_full_size = input_size as u16;
            let stored_compressed_size = compressed.len() as u16;
            compressed_buffer[output_header_offset..output_header_offset + 2].copy_from_slice(&stored_full_size.to_ne_bytes());
            compressed_buffer[output_header_offset + 2..output_header_offset + 4].copy_from_slice(&stored_compressed_size.to_ne_bytes());
            compressed_buffer.extend_from_slice(&compressed);
            Some(compressed_buffer)
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

            let Some(output_buffer) = zlib_decompress(&input_buffer[4..4 + stored_compressed_size as usize], stored_full_size as u64) else {
                println!("[error] Out of memory, could not decompress");
                return false;
            };

            if output_file.write_all(&output_buffer).is_err() || stored_full_size as usize != output_buffer.len() {
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
    zlib_compress(data, 9)
}

/// Not in the C++: the data of a zlib stream, or None when it is damaged.
pub fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    zlib_decompress(data, 1 << 27)
}

/// Not in the C++: the data of a gzip file (a .smwrp recording, one or more members), or None when it is damaged.
pub fn gunzip(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() * 8);
    MultiGzDecoder::new(data).read_to_end(&mut out).ok()?;
    Some(out)
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
        let map = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/maps/0smw.map");
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
