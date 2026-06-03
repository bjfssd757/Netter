use crate::context_worker::{ArgSlice, FlatFunctionSig, FunctionSig};

// --------- IMMUTABLE ---------

pub struct FunctionIterator<'a, const N: usize> {
    buffer: &'a [u8],
    pub(crate) cursor: usize,
}

impl<'a, const N: usize> FunctionIterator<'a, N> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, cursor: 0 }
    }
}

impl<'a, const N: usize> Iterator for FunctionIterator<'a, N> {
    type Item = FunctionSig<'a, N>;

    fn next(&mut self) -> Option<Self::Item> {
        let header_size = size_of::<FlatFunctionSig<N>>();
        let align = align_of::<FlatFunctionSig<N>>();

        loop {
            let remainder = self.cursor % align;
            if remainder != 0 {
                self.cursor += align - remainder;
            }

            if self.cursor + header_size > self.buffer.len() {
                return None;
            }

            unsafe {
                let flat_ptr = self.buffer.as_ptr().add(self.cursor) as *const FlatFunctionSig<N>;
                let flat = &*flat_ptr;

                if flat.name_len == 0 {
                    return None;
                }

                let total_size = header_size + flat.name_len + flat.args_payload_len;
                if self.cursor + total_size > self.buffer.len() {
                    return None;
                }

                let name_start = self.cursor + header_size;
                let name_bytes = &self.buffer[name_start..name_start + flat.name_len];
                let name = std::str::from_utf8_unchecked(name_bytes);

                let payload_start = name_start + flat.name_len;
                let args_payload = &self.buffer[payload_start..payload_start + flat.args_payload_len];

                let sig = FunctionSig {
                    name,
                    ret_ptr: flat.ret_ptr,
                    args_meta: flat.args_meta,
                    args_count: flat.args_count,
                    args_payload,
                };

                self.cursor += total_size;

                return Some(sig);
            }
        }
    }
}

pub trait AsFunction {
    fn with_function<const N: usize>(&self) -> FunctionIterator<N>;
}

impl<const S: usize> AsFunction for [u8; S] {
    fn with_function<const N: usize>(&self) -> FunctionIterator<N> {
        FunctionIterator::<N>::new(self)
    }
}

// --------- MUTABLE ---------

pub struct FunctionIteratorMut<'a, const N: usize> {
    buffer: &'a mut [u8],
    pub(crate) cursor: usize,
}

impl<'a, const N: usize> FunctionIteratorMut<'a, N> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer,
            cursor: 0,
        }
    }
}

pub struct MutFunctionRef<'a, const N: usize> {
    pub flat: &'a mut FlatFunctionSig<N>,
    pub(crate) raw_payload_mut: *mut u8,
}

impl<'a, const N: usize> MutFunctionRef<'a, N> {
    /// delete (zeroize) function in buffer
    pub fn delete(self) {
        let payload_len = self.flat.name_len + self.flat.args_payload_len;
        *self.flat = FlatFunctionSig::default();
        if payload_len > 0 {
            unsafe {
                std::ptr::write_bytes(self.raw_payload_mut, 0, payload_len);
            }
        }
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        let sig_size = size_of::<FlatFunctionSig<N>>();
        unsafe {
            let payload_ptr = (self.flat as *mut FlatFunctionSig<N> as *mut u8)
                .add(sig_size)
                .add(self.flat.name_len);

            std::slice::from_raw_parts_mut(payload_ptr, self.flat.args_payload_len)
        }
    }

    pub fn payload(&self) -> &[u8] {
        let sig_size = size_of::<FlatFunctionSig<N>>();
        unsafe {
            let payload_ptr = (self.flat as *const FlatFunctionSig<N> as *const u8)
                .add(sig_size)
                .add(self.flat.name_len);

            std::slice::from_raw_parts(payload_ptr, self.flat.args_payload_len)
        }
    }

    pub fn arg_meta_mut(&mut self, index: usize) -> Option<&mut ArgSlice> {
        if index < self.flat.args_count {
            Some(&mut self.flat.args_meta[index])
        } else {
            None
        }
    }

    pub fn set_ret_ptr(&mut self, new_ptr: usize) {
        self.flat.ret_ptr = new_ptr;
    }

    pub fn name(&self) -> &str {
        let sig_size = size_of::<FlatFunctionSig<N>>();
        unsafe {
            let name_ptr = (self
                .flat as *const FlatFunctionSig<N> as *const u8)
                .add(sig_size);

            let name_bytes = std::slice::from_raw_parts(name_ptr, self.flat.name_len);
            std::str::from_utf8_unchecked(name_bytes)
        }
    }
}

impl<'a, const N: usize> Iterator for FunctionIteratorMut<'a, N> {
    type Item = MutFunctionRef<'a, N>;

    fn next(&mut self) -> Option<Self::Item> {
        let header_size = size_of::<FlatFunctionSig<N>>();
        let align = align_of::<FlatFunctionSig<N>>();

        loop {
            let remainder = self.cursor % align;
            if remainder != 0 {
                self.cursor += align - remainder;
            }

            if self.cursor + header_size > self.buffer.len() {
                return None;
            }

            unsafe {
                let flat_ptr = self.buffer.as_mut_ptr().add(self.cursor) as *mut FlatFunctionSig<N>;
                let flat = &mut *flat_ptr;

                if flat.name_len == 0 {
                    return None;
                }

                let total_size = header_size + flat.name_len + flat.args_payload_len;
                if self.cursor + total_size > self.buffer.len() {
                    return None;
                }

                let raw_payload_mut = self.buffer.as_mut_ptr().add(self.cursor + header_size);

                self.cursor += total_size;

                return Some(MutFunctionRef {
                    flat: &mut *flat_ptr,
                    raw_payload_mut,
                });
            }
        }
    }
}

pub trait AsFunctionMut {
    fn with_function_mut<const N: usize>(&mut self) -> FunctionIteratorMut<N>;
}

impl<const S: usize> AsFunctionMut for [u8; S] {
    fn with_function_mut<const N: usize>(&mut self) -> FunctionIteratorMut<N> {
        FunctionIteratorMut::<N>::new(self)
    }
}
