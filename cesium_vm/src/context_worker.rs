use crate::iterator::worker_function::{AsFunction, AsFunctionMut};
use crate::iterator::worker_properties::{AsPropertyIterator, AsPropertyIteratorMut};


#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, Default)]
pub struct FlatProperty {
    pub name_len: usize,
    pub value_len: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Property<'a> {
    pub name: &'a str,
    pub value: &'a [u8],
}

impl<'a> Property<'a> {
    pub fn new(name: &'a str, value: &'a [u8]) -> Self {
        Self {
            name, value,
        }
    }
}

#[repr(C, align(8))]
#[derive(Debug, Copy, Clone, Default)]
pub struct ArgSlice {
    pub offset: u32,
    pub len: u32,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct FlatFunctionSig<const ARGS_META_SIZE: usize> {
    pub ret_ptr: usize,
    pub args_meta: [ArgSlice; ARGS_META_SIZE],
    pub args_count: usize,
    pub name_len: usize,
    pub args_payload_len: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct FunctionSig<
    'a,
    const ARGS_META_SIZE: usize,
> {
    pub name: &'a str,
    pub ret_ptr: usize,
    pub args_meta: [ArgSlice; ARGS_META_SIZE],
    pub args_count: usize,
    pub args_payload: &'a [u8],
}

#[derive(Debug, Copy, Clone)]
pub struct NewFunction<'a> {
    pub name: &'a str,
    pub ret_ptr: usize,
    pub args: &'a [&'a [u8]],
}

impl<'a, const ARGS_META_SIZE: usize> Default for FunctionSig<'a, ARGS_META_SIZE> {
    fn default() -> Self {
        Self {
            name: "",
            ret_ptr: 0,
            args_payload: &[0u8],
            args_meta: [ArgSlice::default(); ARGS_META_SIZE],
            args_count: 0,
        }
    }
}

impl<const ARGS_META_SIZE: usize> Default for FlatFunctionSig<ARGS_META_SIZE> {
    fn default() -> Self {
        Self {
            ret_ptr: 0,
            args_meta: [ArgSlice::default(); ARGS_META_SIZE],
            name_len: 0,
            args_payload_len: 0,
            args_count: 0,
        }
    }
}

impl<'a, const ARGS_META_SIZE: usize> FunctionSig<'a, ARGS_META_SIZE> {
    pub fn get_arg(&self, index: usize) -> Option<&'a [u8]> {
        if index >= self.args_count {
            return None;
        }

        let meta = self.args_meta[index];
        let start = meta.offset as usize;
        let end = start + meta.len as usize;

        Some(&self.args_payload[start..end])
    }
}

/// A highly optimized, ultra-lightweight execution context for a worker.
///
/// `WorkerContext` manages stored properties (variables) and functions entirely on the stack
/// within fixed-size byte buffers (`[u8; SIZE]`).
///
/// # Memory Architecture
/// - **Zero Allocations:** All data operations (insertion, reading, updates, deletions) happen
///   directly within static slices without calling the system allocator (`malloc`).
/// - **Hardware-Friendly Alignment:** Structures are automatically laid out along 8-byte boundaries
///   to ensure single-cycle CPU read operations.
/// - **Dynamic Payload Layout:** Strings (names) and variable-length data payloads are packed
///   tightly behind flat headers to maximize L1/L2 cache locality.
///
/// # Type Parameters
/// - `PROPERTY_COUNT`: The total capacity of the properties buffer **in bytes** (e.g., 4096 bytes).
/// - `FUNCTION_COUNT`: The total capacity of the functions buffer **in bytes** (e.g., 12288 bytes).
/// - `FUNCTION_ARGS_COUNT`: The maximum number of arguments metadata slots allocated per single function.
///
/// # Examples
///
/// ```
/// use cesium_vm::context_worker::{WorkerContext, NewFunction};
///
/// // Create context (1 KB)
/// // Max 512 bytes on properties
/// // Max 500 bytes on functions
/// // Max 8 arguments per function
/// let mut ctx = WorkerContext::<516, 500, 8>::new();
///
/// // Store strings or binary state
/// ctx.add_property("env_mode", b"production").unwrap();
///
/// // Read property back
/// if let Some(prop) = ctx.read_property("env_mode") {
///     assert_eq!(prop.value, b"production");
/// }
/// ```
pub struct WorkerContext<
    const PROPERTY_COUNT: usize,
    const FUNCTION_COUNT: usize,
    const FUNCTION_ARGS_SIZE: usize,
> {
    /// Worker stored properties. Used to save variables or other properties.
    pub properties: [u8; PROPERTY_COUNT],
    /// Worker stored functions. Used to save available functions for the worker.
    pub functions: [u8; FUNCTION_COUNT],
}

impl<
    const PROPERTY_COUNT: usize,
    const FUNCTION_COUNT: usize,
    const FUNCTION_ARGS_COUNT: usize,
> WorkerContext<PROPERTY_COUNT, FUNCTION_COUNT, FUNCTION_ARGS_COUNT> {
    pub fn new() -> Self {
        Self {
            properties: [0u8; PROPERTY_COUNT],
            functions: [0u8; FUNCTION_COUNT],
        }
    }

    pub fn compact_properties(&mut self) {
        let header_size = size_of::<FlatProperty>();
        let align = align_of::<FlatProperty>();

        let mut check_cursor = 0;
        let mut has_fragmentation = false;
        let mut found_empty_slot = false;

        unsafe {
            while check_cursor + header_size <= PROPERTY_COUNT {
                let remainder = check_cursor % align;
                if remainder != 0 {
                    check_cursor += align - remainder;
                }
                if check_cursor + header_size > PROPERTY_COUNT { break; }

                let flat = &*(self.properties.as_ptr().add(check_cursor) as *const FlatProperty);

                if flat.name_len == 0 {
                    found_empty_slot = true;
                    check_cursor += header_size;
                } else {
                    if found_empty_slot {
                        has_fragmentation = true;
                        break;
                    }
                    check_cursor += header_size + flat.name_len + flat.value_len;
                }
            }
        }

        if !has_fragmentation {
            return;
        }

        let mut temp_buffer = [0u8; PROPERTY_COUNT];
        let mut read_cursor = 0;
        let mut write_cursor = 0;

        unsafe {
            while read_cursor + header_size <= PROPERTY_COUNT {
                let remainder_read = read_cursor % align;
                if remainder_read != 0 {
                    read_cursor += align - remainder_read;
                }
                if read_cursor + header_size > PROPERTY_COUNT { break; }

                let flat_ptr = self.properties.as_ptr().add(read_cursor) as *const FlatProperty;
                let flat = &*flat_ptr;

                if flat.name_len == 0 {
                    read_cursor += header_size;
                    continue;
                }

                let remainder_write = write_cursor % align;
                if remainder_write != 0 {
                    write_cursor += align - remainder_write;
                }

                let base_dst_ptr = temp_buffer.as_mut_ptr().add(write_cursor);

                std::ptr::copy_nonoverlapping(flat_ptr as *const u8, base_dst_ptr, 1);

                let payload_src = self.properties.as_ptr().add(read_cursor + header_size);
                let payload_dst = base_dst_ptr.add(header_size);
                let payload_len = flat.name_len + flat.value_len;

                std::ptr::copy_nonoverlapping(payload_src, payload_dst, payload_len);

                write_cursor += header_size + payload_len;
                read_cursor += header_size + payload_len;
            }
        }

        self.properties = temp_buffer;
    }

    pub fn compact_functions(&mut self) {
        let header_size = size_of::<FlatFunctionSig<FUNCTION_ARGS_COUNT>>();
        let align = align_of::<FlatFunctionSig<FUNCTION_ARGS_COUNT>>();

        let mut check_cursor = 0;
        let mut has_fragmentation = false;
        let mut found_empty_slot = false;

        unsafe {
            while check_cursor + header_size <= FUNCTION_COUNT {
                let remainder = check_cursor % align;
                if remainder != 0 {
                    check_cursor += align - remainder;
                }
                if check_cursor + header_size > FUNCTION_COUNT { break; }

                let flat = &*(self.functions.as_ptr().add(check_cursor) as *const FlatFunctionSig<FUNCTION_ARGS_COUNT>);

                if flat.name_len == 0 {
                    found_empty_slot = true;
                    check_cursor += header_size;
                } else {
                    if found_empty_slot {
                        has_fragmentation = true;
                        break;
                    }
                    check_cursor += header_size + flat.name_len + flat.args_payload_len;
                }
            }
        }

        if !has_fragmentation {
            return;
        }

        let mut temp_buffer = [0u8; FUNCTION_COUNT];
        let mut read_cursor = 0;
        let mut write_cursor = 0;

        unsafe {
            while read_cursor + header_size <= FUNCTION_COUNT {
                let remainder_read = read_cursor % align;
                if remainder_read != 0 {
                    read_cursor += align - remainder_read;
                }
                if read_cursor + header_size > FUNCTION_COUNT { break; }

                let flat_ptr = self.functions.as_ptr().add(read_cursor) as *const FlatFunctionSig<FUNCTION_ARGS_COUNT>;
                let flat = &*flat_ptr;

                if flat.name_len == 0 {
                    read_cursor += header_size;
                    continue;
                }

                let remainder_write = write_cursor % align;
                if remainder_write != 0 {
                    write_cursor += align - remainder_write;
                }

                let base_dst_ptr = temp_buffer.as_mut_ptr().add(write_cursor);
                std::ptr::copy_nonoverlapping(flat_ptr as *const u8, base_dst_ptr, 1);

                let payload_src = self.functions.as_ptr().add(read_cursor + header_size);
                let payload_dst = base_dst_ptr.add(header_size);
                let payload_len = flat.name_len + flat.args_payload_len;

                std::ptr::copy_nonoverlapping(payload_src, payload_dst, payload_len);

                write_cursor += header_size + payload_len;
                read_cursor += header_size + payload_len;
            }
        }

        self.functions = temp_buffer;
    }

    /// Adds a new property to the worker context.
    ///
    /// # Warning
    ///
    /// Any structures passed to this method as raw bytes must have a C representation
    /// (i.e., `#[repr(C)]`). Always pay attention to the memory representation of the transmitted bytes.
    ///
    /// If bytes obtained from a structure with the default Rust representation (`#[repr(Rust)]`)
    /// are passed to this method, it will not cause an immediate panic. However, any subsequent
    /// reading or casting of these bytes back into a structure will lead to **Undefined Behavior (UB)**
    /// in the reading code.
    pub fn add_property(&mut self, property: Property) -> Result<(), &'static str> {
        let header_size = size_of::<FlatProperty>();
        let align = align_of::<FlatProperty>();

        let mut it = self.properties.with_property_mut();
        while let Some(_) = it.next() {}
        let mut write_cursor = it.cursor;

        let remainder = write_cursor % align;
        if remainder != 0 {
            write_cursor += align - remainder;
        }

        let name_bytes = property.name.as_bytes();
        let total_bytes = header_size + name_bytes.len() + property.value.len();

        if write_cursor + total_bytes > PROPERTY_COUNT {
            return Err("Not enough memory in properties buffer");
        }

        unsafe {
            let base_ptr = self.properties.as_mut_ptr().add(write_cursor);

            let flat = FlatProperty {
                name_len: name_bytes.len(),
                value_len: property.value.len(),
            };
            std::ptr::copy_nonoverlapping(&flat, base_ptr as *mut FlatProperty, 1);

            let name_ptr = base_ptr.add(header_size);
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_ptr, name_bytes.len());

            let val_ptr = name_ptr.add(name_bytes.len());
            std::ptr::copy_nonoverlapping(property.value.as_ptr(), val_ptr, property.value.len());
        }

        Ok(())
    }

    pub fn update_value_property(&mut self, target_name: &str, value: &[u8]) -> Result<bool, &'static str> {
        let mut it = self.properties.with_property_mut();

        while let Some(mut mut_ref) = it.next() {
            if mut_ref.name() == target_name {
                if mut_ref.flat.value_len != value.len() {
                    return Err("Length of new property value not equal old length!");
                }
                mut_ref.value_mut().copy_from_slice(value);
                return Ok(true)
            }
        }
        Ok(false)
    }

    /// Reads the property value as type T.
    ///
    /// # Safety
    ///
    /// This method is unsafe. If the bytes were written without a C language representation
    /// (i.e. `#[repr(C)]`) and did not match the T type, calling the method will result in **undefined behavior (UB)**.
    /// It is highly recommended to use the [Self::read_property()] method instead of [Self::read_property_as()].
    pub unsafe fn read_property_as<T: Copy>(&self, name: &str) -> Option<T> { // <-- Возвращаем T, а не &T
        let prop = self.read_property(name)?;

        if prop.value.len() == size_of::<T>() {
            let value = unsafe { std::ptr::read_unaligned(prop.value.as_ptr() as *const T) };
            Some(value)
        } else {
            None
        }
    }

    pub fn read_property(&self, target_name: &str) -> Option<Property<'_>> {
        let mut it = self.properties.with_property();

        while let Some(prop) = it.next() {
            if prop.name == target_name {
                return Some(prop)
            }
        }
        None
    }

    pub fn delete_property(&mut self, name: &str) -> bool {
        let mut it = self.properties.with_property_mut();

        while let Some(mut_ref) = it.next() {
            if mut_ref.name() == name {
                mut_ref.delete();
                self.compact_properties();
                return true
            }
        }
        false
    }

    pub fn add_function(&mut self, func: NewFunction) -> Result<(), &'static str> {
        if func.args.len() > FUNCTION_ARGS_COUNT {
            return Err("Count of arguments greater then max allow (FUNCTION_ARGS_COUNT)");
        }

        let header_size = size_of::<FlatFunctionSig<FUNCTION_ARGS_COUNT>>();
        let align = align_of::<FlatFunctionSig<FUNCTION_ARGS_COUNT>>();

        let mut write_cursor = 0;
        let buffer_start = self.functions.as_ptr() as usize;
        {
            let mut it = self.functions.with_function_mut();
            while let Some(mut_ref) = it.next() {
                let header_ptr = mut_ref
                    .flat as *const FlatFunctionSig<FUNCTION_ARGS_COUNT> as usize;

                let func_total_size = header_size + mut_ref.flat.name_len + mut_ref.flat.args_payload_len;
                write_cursor = (header_ptr - buffer_start) + func_total_size;
            }
        }

        let remainder = write_cursor % align;
        if remainder != 0 {
            write_cursor += align - remainder;
        }

        let name_bytes = func.name.as_bytes();
        let name_len = name_bytes.len();

        let mut args_payload_len = 0;
        let mut args_meta = [ArgSlice::default(); FUNCTION_ARGS_COUNT];

        for (i, arg) in func.args.iter().enumerate() {
            args_meta[i] = ArgSlice {
                offset: args_payload_len as u32,
                len: arg.len() as u32,
            };
            args_payload_len += arg.len();
        }

        let total_required_size = header_size + name_len + args_payload_len;
        if write_cursor + total_required_size > FUNCTION_COUNT {
            return Err("Not enough memory in buffer for write new function");
        }

        unsafe {
            let base_ptr = self.functions.as_mut_ptr().add(write_cursor);

            let flat_sig = FlatFunctionSig {
                ret_ptr: func.ret_ptr,
                args_meta,
                args_count: func.args.len(),
                name_len,
                args_payload_len,
            };
            std::ptr::copy_nonoverlapping(
                &flat_sig as *const FlatFunctionSig<FUNCTION_ARGS_COUNT> as *const u8,
                base_ptr,
                header_size
            );

            let name_ptr = base_ptr.add(header_size);
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_ptr, name_len);

            let mut current_payload_ptr = name_ptr.add(name_len);
            for arg in func.args.iter() {
                std::ptr::copy_nonoverlapping(arg.as_ptr(), current_payload_ptr, arg.len());
                current_payload_ptr = current_payload_ptr.add(arg.len());
            }
        }

        Ok(())
    }

    pub fn read_all_functions(&self) -> Vec<FunctionSig<'_, FUNCTION_ARGS_COUNT>> {
        let mut it = self.functions.with_function();
        let mut res = Vec::new();

        while let Some(func) = it.next() {
            res.push(func);
        }

        res
    }

    pub fn read_function(&self, name: &str) -> Option<FunctionSig<'_, FUNCTION_ARGS_COUNT>> {
        let mut it = self.functions.with_function();

        while let Some(func) = it.next() {
            if func.name == name {
                return Some(func)
            }
        }
        None
    }

    pub fn delete_function(&mut self, target_name: &str) -> bool {
        let mut it = self.functions
            .with_function_mut::<FUNCTION_ARGS_COUNT>();

        while let Some(mut_ref) = it.next() {
            if mut_ref.name() == target_name {
                mut_ref.delete();
                self.compact_functions();
                return true
            }
        }
        false
    }

    pub fn update_function_ret_ptr(&mut self, target_name: &str, new_ptr: usize) -> bool {
        let mut it = self.functions
            .with_function_mut::<FUNCTION_ARGS_COUNT>();

        while let Some(mut mut_ref) = it.next() {
            if mut_ref.name() == target_name {
                mut_ref.set_ret_ptr(new_ptr);
                return true
            }
        }
        false
    }

    pub fn update_function_args_payload(
        &mut self, target_name: &str,
        new_payload: &[u8]
    ) -> Result<bool, &'static str> {
        let mut it = self.functions
            .with_function_mut::<FUNCTION_ARGS_COUNT>();

        while let Some(mut mut_ref) = it.next() {
            if mut_ref.name() == target_name {
                if mut_ref.flat.args_payload_len != new_payload.len() {
                    return Err("Length of new payload not equal old payload length!");
                }
                mut_ref.payload_mut().copy_from_slice(new_payload);
                return Ok(true)
            }
        }
        Ok(false)
    }
}


// ---------- TESTS ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestData {
        id: u32,
        value: u32,
    }

    #[test]
    fn test_property() {
        let mut ctx = WorkerContext::<516, 500, 8>::new();

        let prop1 = Property::new("var1", b"hello");
        let prop2 = Property::new("var2", b"world");

        ctx.add_property(prop1).unwrap();
        ctx.add_property(prop2).unwrap();

        let p1 = ctx.read_property("var1").unwrap();
        assert_eq!(p1.value, b"hello");

        let p2 = ctx.read_property("var2").unwrap();
        assert_eq!(p2.value, b"world");

        assert!(ctx.update_value_property("var1", b"bytes").unwrap());
        let prop1_updated = ctx.read_property("var1").unwrap();
        assert_eq!(prop1_updated.value, b"bytes");

        assert!(ctx.update_value_property("var1", b"long_bytes").is_err());

        assert!(ctx.delete_property("var1"));
        assert!(ctx.read_property("var1").is_none());

        assert!(ctx.read_property("var2").is_some());
    }

    #[test]
    fn test_property_read_as_struct() {
        let mut ctx = WorkerContext::<256, 256, 4>::new();
        let data = TestData { id: 42, value: 100 };

        let bytes = unsafe {
            std::slice::from_raw_parts(
                &data as *const TestData as *const u8,
                size_of::<TestData>(),
            )
        };

        let prop = Property::new("struct_prop", bytes);

        ctx.add_property(prop).unwrap();

        unsafe {
            let read_data: TestData = ctx.read_property_as("struct_prop").unwrap();
            assert_eq!(read_data, data);
        }
    }

    #[test]
    fn test_function() {
        let mut ctx = WorkerContext::<256, 512, 4>::new();

        let arg1 = b"abc";
        let arg2 = b"de";
        let func = NewFunction {
            name: "test_fn",
            ret_ptr: 0xDEADBEEF,
            args: &[arg1, arg2],
        };

        ctx.add_function(func).unwrap();

        let read_fn = ctx.read_function("test_fn").unwrap();
        assert_eq!(read_fn.ret_ptr, 0xDEADBEEF);
        assert_eq!(read_fn.args_count, 2);

        assert_eq!(read_fn.get_arg(0).unwrap(), b"abc");
        assert_eq!(read_fn.get_arg(1).unwrap(), b"de");

        let new_payload = b"xyz12";
        assert!(ctx.update_function_args_payload("test_fn", new_payload).unwrap());

        assert!(ctx.delete_function("test_fn"));
        assert!(ctx.read_function("test_fn").is_none());
    }

    #[test]
    fn test_buffer_overflow() {
        let mut ctx = WorkerContext::<16, 16, 2>::new();

        let prop = Property::new("a", b"1");
        assert!(ctx.add_property(prop).is_err());
    }
}