//! Port of src/common/FileIO.cpp

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};

/// C++ `throw std::runtime_error(...)`; caught by `catch_unwind` at the C++ `catch` sites.
pub fn throw_runtime_error(what: &str) -> ! {
    std::panic::panic_any(what.to_string())
}

enum Handle {
    R(BufReader<File>),
    W(BufWriter<File>),
    M(Cursor<Vec<u8>>),
}

pub struct BinaryFile {
    m_path: String,
    fp: Option<Handle>,
}

extern "C" {
    fn strerror(errnum: i32) -> *const std::ffi::c_char;
}

/// The C `std::strerror` text for an IO error that `std::ferror` would flag (not end of file).
fn system_message(err: &std::io::Error) -> Option<String> {
    let code = err.raw_os_error()?;
    Some(unsafe { std::ffi::CStr::from_ptr(strerror(code)).to_string_lossy().into_owned() })
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
        BinaryFile { m_path: path.to_string(), fp }
    }

    /// Not in the C++: reads from or writes to memory (replay checkpoints, smw/checkpoint.rs).
    pub fn memory(name: &str, bytes: Vec<u8>) -> Self {
        BinaryFile { m_path: name.to_string(), fp: Some(Handle::M(Cursor::new(bytes))) }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self.fp {
            Some(Handle::M(m)) => m.into_inner(),
            _ => Vec::new(),
        }
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
            Some(Handle::M(m)) => m.set_position(0),
            None => {}
        }
    }

    pub fn pos(&mut self) -> i64 {
        match &mut self.fp {
            Some(Handle::R(r)) => r.stream_position().map(|p| p as i64).unwrap_or(-1),
            Some(Handle::W(w)) => w.stream_position().map(|p| p as i64).unwrap_or(-1),
            Some(Handle::M(m)) => m.position() as i64,
            None => 0,
        }
    }

    fn fread_or_exception(&mut self, buf: &mut [u8]) {
        let pos = self.pos();
        let result = match &mut self.fp {
            Some(Handle::R(r)) => r.read_exact(buf),
            Some(Handle::M(m)) => m.read_exact(buf),
            _ => Err(std::io::Error::from_raw_os_error(9)),
        };
        if let Err(err) = result {
            let mut msg = format!(
                "File read error in {}\n\
                 Tried to read {} bytes at position {}, but failed\n\
                 The file might be damaged, or it's not in the expected format",
                self.m_path,
                buf.len(),
                pos
            );
            if let Some(system) = system_message(&err) {
                msg += "\nSystem message: ";
                msg += &system;
            }
            throw_runtime_error(&msg);
        }
    }

    fn fwrite_or_exception(&mut self, buf: &[u8]) {
        let pos = self.pos();
        let result = match &mut self.fp {
            Some(Handle::W(w)) => w.write_all(buf),
            Some(Handle::M(m)) => m.write_all(buf),
            _ => Err(std::io::Error::from_raw_os_error(9)),
        };
        if let Err(err) = result {
            let mut msg = format!("File write error in {}\nTried to write {} bytes at position {}, but failed", self.m_path, buf.len(), pos);
            if let Some(system) = system_message(&err) {
                msg += "\nSystem message: ";
                msg += &system;
            }
            throw_runtime_error(&msg);
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

    /// Writes an i32 that tells the byte length of the string data *including*
    /// a terminating null byte, then the text data itself, plus a null byte
    pub fn write_string_long(&mut self, string: &str) {
        let mut bytes = string.as_bytes().to_vec();
        if bytes.len() > 255 {
            let msg = format!(
                "File write error in {}\nTried to write a text that would take {} bytes, which is too long",
                self.m_path,
                bytes.len()
            );
            throw_runtime_error(&msg);
        }

        self.write_i32(bytes.len() as i32 + 1);
        bytes.push(0);
        self.fwrite_or_exception(&bytes);
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

    /// Uses 32 bits to store the length of the string, then the text data,
    /// including a terminating null byte. Reads at most `maxlen` bytes and always drops the last one.
    pub fn read_string_long(&mut self, maxlen: usize) -> String {
        let stored_len = self.read_i32();
        if stored_len <= 0 {
            return String::new();
        }

        let data_len = (stored_len as usize).min(maxlen);
        if data_len == 0 {
            return String::new();
        }

        let mut text = vec![0u8; data_len];
        self.fread_or_exception(&mut text);

        // NOTE: The stored text always includes a terminating null byte
        text.pop();

        cstr_bytes_to_string(&text)
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
            f.write_string_long("world!");
            f.write_string_long(&"x".repeat(255));
            f.write_string_long("a\0b");
            f.write_string_long("truncated");
            f.write_bool(true);
        }
        let mut f = BinaryFile::new(p, "rb");
        assert_eq!(f.read_i32(), -5);
        assert_eq!(f.read_i16(), 300);
        assert_eq!(f.read_float(), 1.5);
        assert_eq!(f.read_string_long(128), "world!");
        assert_eq!(f.read_string_long(512), "x".repeat(255));
        assert_eq!(f.read_string_long(128), "a\0b");
        // Reads only maxlen bytes (dropping the last), leaving the rest of the text unread.
        assert_eq!(f.read_string_long(4), "tru");
        let mut rest = [0u8; 6];
        f.read_raw(&mut rest);
        assert_eq!(&rest, b"cated\0");
        assert!(f.read_bool());
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.read_i32()));
        assert!(r.is_err());
        let mut w = BinaryFile::new(p, "wb");
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.write_string_long(&"y".repeat(256))));
        assert!(r.is_err());
        let _ = std::fs::remove_file(&path);
    }
}
