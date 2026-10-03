//! Port of src/smw/ObjectContainer.cpp

use crate::common::moving_object_types::MovingObjectType;
use crate::common::object_base::{CObjectTrait, ObjectType};
use crate::globals::{Aliased, Ptr};

pub const MAXOBJECTS: usize = 300;

/// `std::vector<std::unique_ptr<CObject>>` with `reserve(MAXOBJECTS)`; the container owns each
/// object and deletes it in `clean` / `clean_dead_objects`.
pub struct CObjectContainer {
    m_list: Vec<Ptr<dyn CObjectTrait>>,
    pub _alias: Aliased,
}

impl Default for CObjectContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl CObjectContainer {
    pub const fn new() -> Self {
        CObjectContainer { m_list: Vec::new(), _alias: Aliased::new() }
    }

    /// libc++ `reserve(300)` gives exactly 300 and `add` never grows past it.
    fn capacity(&self) -> usize {
        MAXOBJECTS
    }

    pub fn clean(&mut self) {
        for obj in std::mem::take(&mut self.m_list) {
            obj.delete();
        }
    }

    pub fn clean_dead_objects(&mut self) {
        let mut kept: Vec<Ptr<dyn CObjectTrait>> = Vec::with_capacity(self.m_list.len());
        for obj in std::mem::take(&mut self.m_list) {
            if obj.is_dead() {
                obj.delete();
            } else {
                kept.push(obj);
            }
        }
        self.m_list = kept;
    }

    /// `add(new T(...))`: takes ownership of a `Ptr::new_box` object.
    pub fn add<T: CObjectTrait>(&mut self, ptr: Ptr<T>) -> bool {
        self.add_dyn(Ptr::from_raw(ptr.as_ptr() as *mut dyn CObjectTrait))
    }

    pub fn add_dyn(&mut self, ptr: Ptr<dyn CObjectTrait>) -> bool {
        if self.m_list.len() + 1 >= self.capacity() {
            //printf("eyecandy list full!\n");
            ptr.delete(); // otherwise memory leak!
            return false;
        }
        self.m_list.push(ptr);
        true
    }

    /// The C++ range-for captures `end()` once: objects added during the loop wait for the next frame.
    pub fn update(&self) {
        let this = self as *const CObjectContainer;
        let n = self.m_list.len();
        for i in 0..n {
            let mut obj = unsafe { (&(*this).m_list)[i] };
            obj.update();
        }
    }

    pub fn draw(&self) {
        let this = self as *const CObjectContainer;
        let n = self.m_list.len();
        for i in 0..n {
            let mut obj = unsafe { (&(*this).m_list)[i] };
            obj.draw();
        }
    }

    pub fn get_closest_object(&self, ix: i16, iy: i16, objectType: ObjectType) -> f32 {
        let mut minDist = f32::MAX;

        for obj in self.m_list.iter() {
            let mut obj = *obj;
            if obj.get_object_type() != objectType {
                continue;
            }

            let x: i16 = (obj.x() - ix as i32) as i16;
            let y: i16 = (obj.y() - iy as i32) as i16;

            let dist = (x as i32 * x as i32 + y as i32 * y as i32) as f32;
            if dist < minDist {
                minDist = dist;
            }
        }

        minDist.sqrt()
    }

    pub fn get_closest_moving_object(&self, ix: i16, iy: i16, movingObjectType: MovingObjectType) -> f32 {
        let mut minDist = f32::MAX;

        for obj in self.m_list.iter() {
            let mut obj = *obj;
            match obj.as_io_moving_object() {
                Some(m) if m.get_moving_object_type() == movingObjectType => {}
                _ => continue,
            }

            let x: i16 = (obj.x() - ix as i32) as i16;
            let y: i16 = (obj.y() - iy as i32) as i16;

            let dist = (x as i32 * x as i32 + y as i32 * y as i32) as f32;
            if dist < minDist {
                minDist = dist;
            }
        }

        minDist.sqrt()
    }

    pub fn count_types(&self, r#type: ObjectType) -> usize {
        let mut count = 0;

        for obj in self.m_list.iter() {
            let mut obj = *obj;
            if obj.get_object_type() == r#type {
                count += 1;
            }
        }

        count
    }

    pub fn count_moving_types(&self, r#type: MovingObjectType) -> usize {
        let mut count = 0;

        for obj in self.m_list.iter() {
            let mut obj = *obj;
            if let Some(m) = obj.as_io_moving_object() {
                if m.get_moving_object_type() == r#type {
                    count += 1;
                }
            }
        }

        count
    }

    pub fn list(&self) -> &Vec<Ptr<dyn CObjectTrait>> {
        &self.m_list
    }
}
