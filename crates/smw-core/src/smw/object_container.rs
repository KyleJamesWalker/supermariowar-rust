//! Port of src/smw/ObjectContainer.cpp

use crate::game::Game;
use crate::common::moving_object_types::MovingObjectType;
use crate::common::object_base::{CObjectTrait, ObjectType};
use crate::globals::{Aliased, Ptr};

pub const MAXOBJECTS: usize = 300;

/// An object in one of `Game`'s containers. A handle to a deleted object finds nothing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ObjectId {
    container: u8,
    slot: u16,
    generation: u32,
}

impl ObjectId {
    /// No object: what a null object pointer was in the C++.
    pub const NONE: ObjectId = ObjectId { container: u8::MAX, slot: 0, generation: 0 };

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }

    pub fn container(self) -> u8 {
        self.container
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::NONE
    }
}

/// `std::vector<std::unique_ptr<CObject>>` with `reserve(MAXOBJECTS)`; the container owns each
/// object and deletes it in `clean` / `clean_dead_objects`.
pub struct CObjectContainer {
    m_list: Vec<Ptr<dyn CObjectTrait>>,
    /// The container's number in its objects' `ObjectId`s.
    index: u8,
    /// Each object by its `CObject::id` slot. A freed slot's generation moves on.
    slots: Vec<Option<Ptr<dyn CObjectTrait>>>,
    generations: Vec<u32>,
    pub _alias: Aliased,
}

impl Clone for CObjectContainer {
    /// Copies every object, including reserved ones; handles into the original find the copies.
    fn clone(&self) -> Self {
        let slots: Vec<Option<Ptr<dyn CObjectTrait>>> = self.slots.iter().map(|s| s.map(|obj| obj.clone_object())).collect();
        let copy_of = |obj: &Ptr<dyn CObjectTrait>| {
            let slot = obj.id.slot as usize;
            match self.slots.get(slot) {
                Some(Some(listed)) if listed == obj => slots[slot].unwrap(),
                _ => obj.clone_object(),
            }
        };
        let m_list = self.m_list.iter().map(copy_of).collect();
        CObjectContainer { m_list, index: self.index, slots, generations: self.generations.clone(), _alias: Aliased::new() }
    }
}

impl Drop for CObjectContainer {
    fn drop(&mut self) {
        self.clean();
        for obj in std::mem::take(&mut self.slots).into_iter().flatten() {
            obj.delete();
        }
    }
}

impl CObjectContainer {
    pub const fn new(index: u8) -> Self {
        CObjectContainer { m_list: Vec::new(), index, slots: Vec::new(), generations: Vec::new(), _alias: Aliased::new() }
    }

    pub fn get(&self, id: ObjectId) -> Option<Ptr<dyn CObjectTrait>> {
        let slot = id.slot as usize;
        if id.container != self.index || slot >= self.slots.len() || self.generations[slot] != id.generation {
            return None;
        }
        self.slots[slot]
    }

    fn assign(&mut self, mut obj: Ptr<dyn CObjectTrait>) {
        let slot = match self.slots.iter().position(Option::is_none) {
            Some(slot) => slot,
            None => {
                self.slots.push(None);
                self.generations.push(0);
                self.slots.len() - 1
            }
        };
        self.slots[slot] = Some(obj);
        obj.id = ObjectId { container: self.index, slot: slot as u16, generation: self.generations[slot] };
    }

    fn release(&mut self, obj: Ptr<dyn CObjectTrait>) {
        let slot = obj.id.slot as usize;
        if self.get(obj.id).is_some() {
            self.slots[slot] = None;
            self.generations[slot] = self.generations[slot].wrapping_add(1);
        }
        obj.delete();
    }

    /// libc++ `reserve(300)` gives exactly 300 and `add` never grows past it.
    fn capacity(&self) -> usize {
        MAXOBJECTS
    }

    pub fn clean(&mut self) {
        for obj in std::mem::take(&mut self.m_list) {
            self.release(obj);
        }
    }

    pub fn clean_dead_objects(&mut self) {
        let mut kept: Vec<Ptr<dyn CObjectTrait>> = Vec::with_capacity(self.m_list.len());
        for obj in std::mem::take(&mut self.m_list) {
            if obj.is_dead() {
                self.release(obj);
            } else {
                kept.push(obj);
            }
        }
        self.m_list = kept;
    }

    /// `add(new T(...))`: takes ownership of a `Ptr::new_box` object.
    pub fn add<T: CObjectTrait>(&mut self, g: &mut Game, ptr: Ptr<T>) -> bool {
        self.add_dyn(g, Ptr::from_raw(ptr.as_ptr() as *mut dyn CObjectTrait))
    }

    /// `add`, returning the object's handle, or `ObjectId::NONE` when the container was full.
    pub fn add_id<T: CObjectTrait>(&mut self, g: &mut Game, ptr: Ptr<T>) -> ObjectId {
        let obj = Ptr::from_raw(ptr.as_ptr() as *mut dyn CObjectTrait);
        if self.add_dyn(g, obj) { obj.id } else { ObjectId::NONE }
    }

    /// Gives a `Ptr::new_box` object its handle before `add`, for an item a player carries first.
    pub fn reserve<T: CObjectTrait>(&mut self, ptr: Ptr<T>) -> ObjectId {
        let obj = Ptr::from_raw(ptr.as_ptr() as *mut dyn CObjectTrait);
        self.assign(obj);
        obj.id
    }

    /// Deletes a reserved object that was never added.
    pub fn unreserve<T: CObjectTrait>(&mut self, ptr: Ptr<T>) {
        self.release(Ptr::from_raw(ptr.as_ptr() as *mut dyn CObjectTrait));
    }

    pub fn add_dyn(&mut self, g: &mut Game, ptr: Ptr<dyn CObjectTrait>) -> bool {
        if self.m_list.len() + 1 >= self.capacity() {
            //printf("eyecandy list full!\n");
            self.release(ptr); // otherwise memory leak!
            return false;
        }
        crate::smw::net_random::note_added(g, ptr);
        if self.get(ptr.id).is_none() {
            self.assign(ptr);
        }
        self.m_list.push(ptr);
        true
    }

    /// The C++ range-for captures `end()` once: objects added during the loop wait for the next frame.
    pub fn update(&self, g: &mut Game) {
        let this = self as *const CObjectContainer;
        let n = self.m_list.len();
        for i in 0..n {
            let mut obj = unsafe { (&(*this).m_list)[i] };
            obj.update(g);
        }
    }

    pub fn draw(&self, g: &mut Game) {
        let this = self as *const CObjectContainer;
        let n = self.m_list.len();
        for i in 0..n {
            let mut obj = unsafe { (&(*this).m_list)[i] };
            crate::common::gfx::display_list::set_owner(crate::common::gfx::display_list::owner_of(obj.as_ptr()));
            obj.draw(g);
        }
        crate::common::gfx::display_list::set_owner(crate::common::gfx::display_list::Owner::None);
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
