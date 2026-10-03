//! Port of src/common/FileIO.cpp

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};

/// C++ `throw std::runtime_error(...)`; caught by `catch_unwind` at the C++ `catch` sites.
pub fn throw_runtime_error(what: &str) -> ! {
    std::panic::panic_any(what.to_string())
}

enum Handle {
    R(BufReader<File>),
    W(BufWriter<File>),
}

pub struct BinaryFile {
    fp: Option<Handle>,
}

impl BinaryFile {
    pub fn new(path: &str, options: &str) -> Self {
        let fp = if options.contains('r') {
            File::open(path).ok().map(|f| Handle::R(BufReader::new(f)))
        } else if options.contains('a') {
            OpenOptions::new().append(true).create(true).open(path).ok().map(|f| Handle::W(BufWriter::new(f)))
        } else {
            File::create(path).ok().map(|f| Handle::W(BufWriter::new(f)))
        };
        BinaryFile { fp }
    }

    pub fn is_open(&self) -> bool {
        self.fp.is_some()
    }

    pub fn rewind(&mut self) {
        match &mut self.fp {
            Some(Handle::R(r)) => {
                let _ = r.seek(SeekFrom::Start(0));
            }
            Some(Handle::W(w)) => {
                let _ = w.seek(SeekFrom::Start(0));
            }
            None => {}
        }
    }

    fn fread_or_exception(&mut self, buf: &mut [u8]) {
        let ok = match &mut self.fp {
            Some(Handle::R(r)) => r.read_exact(buf).is_ok(),
            _ => false,
        };
        if !ok {
            throw_runtime_error("File read error");
        }
    }

    fn fwrite_or_exception(&mut self, buf: &[u8]) {
        let ok = match &mut self.fp {
            Some(Handle::W(w)) => w.write_all(buf).is_ok(),
            _ => false,
        };
        if !ok {
            throw_runtime_error("File write error");
        }
    }

    pub fn write_i8(&mut self, value: i8) {
        self.fwrite_or_exception(&value.to_le_bytes());
    }

    pub fn write_u8(&mut self, value: u8) {
        self.fwrite_or_exception(&value.to_le_bytes());
    }

    pub fn write_i16(&mut self, value: i16) {
        self.fwrite_or_exception(&value.to_le_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.fwrite_or_exception(&value.to_le_bytes());
    }

    pub fn write_bool(&mut self, value: bool) {
        self.write_u8(value as u8);
    }

    pub fn write_float(&mut self, value: f32) {
        self.fwrite_or_exception(&value.to_le_bytes());
    }

    pub fn write_string(&mut self, string: &str) {
        let bytes = string.as_bytes();
        let mut len = bytes.len() as i32 + 1;
        if len > 255 {
            len = 255;
        }

        self.write_u8(len as u8);
        self.write_cstr_bytes(bytes, len as usize);
    }

    pub fn write_string_long(&mut self, string: &str) {
        let bytes = string.as_bytes();
        let mut len = bytes.len() as i32 + 1;
        if len > 255 {
            len = 255;
        }

        self.write_i32(len);
        self.write_cstr_bytes(bytes, len as usize);
    }

    fn write_cstr_bytes(&mut self, bytes: &[u8], len: usize) {
        let mut buf = bytes.to_vec();
        buf.push(0);
        buf.truncate(len);
        self.fwrite_or_exception(&buf);
    }

    pub fn write_raw(&mut self, source: &[u8]) {
        self.fwrite_or_exception(source);
    }

    pub fn read_u8(&mut self) -> u8 {
        let mut b = [0u8; 1];
        self.fread_or_exception(&mut b);
        b[0]
    }

    pub fn read_i8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    pub fn read_bool(&mut self) -> bool {
        self.read_u8() != 0
    }

    pub fn read_i16(&mut self) -> i16 {
        let mut b = [0u8; 2];
        self.fread_or_exception(&mut b);
        i16::from_le_bytes(b)
    }

    pub fn read_i16_array(&mut self, target: &mut [i16]) {
        let mut b = vec![0u8; target.len() * 2];
        self.fread_or_exception(&mut b);
        for (i, t) in target.iter_mut().enumerate() {
            *t = i16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
        }
    }

    pub fn read_i32(&mut self) -> i32 {
        let mut b = [0u8; 4];
        self.fread_or_exception(&mut b);
        i32::from_le_bytes(b)
    }

    pub fn read_i32_array(&mut self, target: &mut [i32]) {
        let mut b = vec![0u8; target.len() * 4];
        self.fread_or_exception(&mut b);
        for (i, t) in target.iter_mut().enumerate() {
            *t = i32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
        }
    }

    pub fn read_float(&mut self) -> f32 {
        let mut b = [0u8; 4];
        self.fread_or_exception(&mut b);
        f32::from_le_bytes(b)
    }

    /// Reads into a C `char[size]` buffer and returns its contents up to the first NUL.
    pub fn read_string(&mut self, size: usize) -> String {
        let len = self.read_u8() as usize;
        if len == 0 {
            return String::new();
        }
        self.read_cstr_body(len, size)
    }

    pub fn read_string_long(&mut self, size: usize) -> String {
        let len = self.read_i32();
        if len <= 0 {
            return String::new();
        }
        self.read_cstr_body(len as usize, size)
    }

    fn read_cstr_body(&mut self, len: usize, size: usize) -> String {
        let mut string = vec![0u8; len];
        self.fread_or_exception(&mut string);
        string[len - 1] = 0;

        let mut end = string.iter().position(|&c| c == 0).unwrap_or(len);
        if end > size - 1 {
            end = size - 1;
        }
        cstr_bytes_to_string(&string[..end])
    }

    pub fn read_raw(&mut self, target: &mut [u8]) {
        self.fread_or_exception(target);
    }
}

/// C strings in data files are Latin-1/ASCII bytes; map each byte to one char so lengths stay in bytes for ASCII.
pub fn cstr_bytes_to_string(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let path = std::env::temp_dir().join(format!("smw_fileio_{}.bin", std::process::id()));
        let p = path.to_str().unwrap();
        {
            let mut f = BinaryFile::new(p, "wb");
            f.write_i32(-5);
            f.write_i16(300);
            f.write_float(1.5);
            f.write_string("hello");
            f.write_string_long("world!");
            f.write_bool(true);
        }
        let mut f = BinaryFile::new(p, "rb");
        assert_eq!(f.read_i32(), -5);
        assert_eq!(f.read_i16(), 300);
        assert_eq!(f.read_float(), 1.5);
        assert_eq!(f.read_string(4), "hel");
        assert_eq!(f.read_string_long(128), "world!");
        assert!(f.read_bool());
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.read_i32()));
        assert!(r.is_err());
        let _ = std::fs::remove_file(&path);
    }
}
