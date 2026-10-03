//! Port of src/common/util/Grid.h

#[derive(Clone, Debug, Default)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    pub fn new(w: usize, h: usize) -> Self {
        Grid { width: w, height: h, data: vec![T::default(); w * h] }
    }

    pub fn fill(&mut self, val: &T) {
        self.data = vec![val.clone(); self.width * self.height];
    }
}

impl<T> Grid<T> {
    pub fn cols(&self) -> usize {
        self.width
    }
    pub fn rows(&self) -> usize {
        self.height
    }
    pub fn empty(&self) -> bool {
        self.data.is_empty()
    }
    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.height = 0;
        self.width = 0;
    }

    pub fn swap(&mut self, other: &mut Grid<T>) {
        std::mem::swap(self, other);
    }

    pub fn at(&self, x: usize, y: usize) -> &T {
        &self.data[y * self.width + x]
    }
    pub fn at_mut(&mut self, x: usize, y: usize) -> &mut T {
        &mut self.data[y * self.width + x]
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }
}
