use std::mem::{size_of, align_of};
use crate::context_worker::{FlatProperty, Property};

// --------- MUTABLE ----------

pub struct MutPropertyRef<'a> {
    pub flat: &'a mut FlatProperty,
    pub(crate) raw_payload_mut: *mut u8,
}

impl<'a> MutPropertyRef<'a> {
    pub fn name(&self) -> &str {
        let header_size = size_of::<FlatProperty>();
        unsafe {
            let name_ptr = (self.flat as *const FlatProperty as *const u8).add(header_size);
            let name_bytes = std::slice::from_raw_parts(name_ptr, self.flat.name_len);
            std::str::from_utf8_unchecked(name_bytes)
        }
    }

    pub fn value_mut(&mut self) -> &mut [u8] {
        unsafe {
            let value_ptr = self.raw_payload_mut.add(self.flat.name_len);
            std::slice::from_raw_parts_mut(value_ptr, self.flat.value_len)
        }
    }

    pub fn delete(self) {
        let total_payload_len = self.flat.name_len + self.flat.value_len;
        if total_payload_len > 0 {
            unsafe {
                std::ptr::write_bytes(self.raw_payload_mut, 0, total_payload_len);
            }
        }
        *self.flat = FlatProperty::default();
    }
}

pub struct PropertyIteratorMut<'a> {
    buffer: &'a mut [u8],
    pub(crate) cursor: usize,
}

impl<'a> PropertyIteratorMut<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer,
            cursor: 0,
        }
    }
}

impl<'a> Iterator for PropertyIteratorMut<'a> {
    type Item = MutPropertyRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let header_size = size_of::<FlatProperty>();
        let align = align_of::<FlatProperty>();

        loop {
            let remainder = self.cursor % align;
            if remainder != 0 {
                self.cursor += align - remainder;
            }

            if self.cursor + header_size > self.buffer.len() {
                return None;
            }

            unsafe {
                let flat_ptr = self.buffer.as_mut_ptr().add(self.cursor) as *mut FlatProperty;
                let flat = &mut *flat_ptr;

                if flat.name_len == 0 {
                    return None;
                }

                let total_size = header_size + flat.name_len + flat.value_len;
                if self.cursor + total_size > self.buffer.len() {
                    return None;
                }

                let raw_payload_mut = self.buffer.as_mut_ptr().add(self.cursor + header_size);

                self.cursor += total_size;

                return Some(MutPropertyRef {
                    flat: &mut *flat_ptr,
                    raw_payload_mut,
                });
            }
        }
    }
}

pub trait AsPropertyIteratorMut {
    fn with_property_mut(&mut self) -> PropertyIteratorMut;
}

impl<const N: usize> AsPropertyIteratorMut for [u8; N] {
    fn with_property_mut(&mut self) -> PropertyIteratorMut {
        PropertyIteratorMut::new(self)
    }
}

// --------- IMMUTABLE ----------

pub struct PropertyIterator<'a> {
    buffer: &'a [u8],
    pub(crate) cursor: usize,
}

impl<'a> PropertyIterator<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            cursor: 0,
        }
    }
}

impl<'a> Iterator for PropertyIterator<'a> {
    type Item = Property<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let header_size = size_of::<FlatProperty>();
        let align = align_of::<FlatProperty>();

        loop {
            let remainder = self.cursor % align;
            if remainder != 0 {
                self.cursor += align - remainder;
            }

            if self.cursor + header_size > self.buffer.len() {
                return None;
            }

            unsafe {
                let flat_ptr = self.buffer.as_ptr().add(self.cursor) as *const FlatProperty;
                let flat = &*flat_ptr;

                if flat.name_len == 0 {
                    return None;
                }

                let total_size = header_size + flat.name_len + flat.value_len;
                if self.cursor + total_size > self.buffer.len() {
                    return None;
                }

                let name_start = self.cursor + header_size;
                let name_bytes = &self.buffer[name_start..name_start + flat.name_len];
                let name = std::str::from_utf8_unchecked(name_bytes);

                let value_start = name_start + flat.name_len;
                let value = &self.buffer[value_start..value_start + flat.value_len];

                let prop = Property { name, value };

                self.cursor += total_size;

                return Some(prop);
            }
        }
    }
}

pub trait AsPropertyIterator {
    fn with_property(&self) -> PropertyIterator;
}

impl<const N: usize> AsPropertyIterator for [u8; N] {
    fn with_property(&self) -> PropertyIterator {
        PropertyIterator::new(self)
    }
}
