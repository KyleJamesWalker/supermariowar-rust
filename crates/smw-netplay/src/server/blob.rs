//! Port of src/server/Blob.cpp

#[derive(Default)]
pub struct Blob {
    data: Vec<u8>,
}

impl Blob {
    pub fn new() -> Self {
        Blob { data: Vec::new() }
    }

    pub fn empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn replace_with(&mut self, src: &[u8]) {
        self.free();
        self.data = src.to_vec();
    }

    pub fn free(&mut self) {
        self.data = Vec::new();
    }

    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    pub fn get_data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn get_size(&self) -> usize {
        self.data.len()
    }
}
