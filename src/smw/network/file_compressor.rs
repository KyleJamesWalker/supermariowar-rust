//! Port of src/smw/network/FileCompressor.cpp

use libz_sys::{compress, compressBound, uLong, uncompress, Z_OK};
use std::io::{Read, Seek, SeekFrom, Write};

const COMPRESSION_SIZE_LIMIT: u64 = 20000;

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
}
