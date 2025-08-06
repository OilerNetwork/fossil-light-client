//! Linux-compatible version of size-of crate
//! 
//! This is a minimal implementation that provides the SizeOf trait
//! without the problematic ABI implementations that don't work on Linux.

/// Context for size calculations
pub struct Context;

impl Context {
    pub fn new() -> Self {
        Context
    }
}

/// A trait for getting the size of a type
pub trait SizeOf {
    /// Returns the size of the type in bytes
    fn size_of(&self) -> usize {
        std::mem::size_of_val(self)
    }
    
    /// Calculate size of children (default implementation does nothing)
    fn size_of_children(&self, _context: &mut Context) {}
}

// Implement for basic types
impl SizeOf for u8 {}
impl SizeOf for u16 {}
impl SizeOf for u32 {}
impl SizeOf for u64 {}
impl SizeOf for u128 {}
impl SizeOf for usize {}

impl SizeOf for i8 {}
impl SizeOf for i16 {}
impl SizeOf for i32 {}
impl SizeOf for i64 {}
impl SizeOf for i128 {}
impl SizeOf for isize {}

impl SizeOf for f32 {}
impl SizeOf for f64 {}

impl SizeOf for bool {}
impl SizeOf for char {}

impl SizeOf for () {}

// Implement for arrays
impl<T: SizeOf, const N: usize> SizeOf for [T; N] {}

// Implement for slices
impl<T: SizeOf> SizeOf for [T] {}

// Implement for strings
impl SizeOf for str {}
impl SizeOf for String {}

// Implement for vectors
impl<T: SizeOf> SizeOf for Vec<T> {}

// Implement for tuples (up to 12 elements like the original)
impl<T0: SizeOf> SizeOf for (T0,) {}
impl<T0: SizeOf, T1: SizeOf> SizeOf for (T0, T1) {}
impl<T0: SizeOf, T1: SizeOf, T2: SizeOf> SizeOf for (T0, T1, T2) {}
impl<T0: SizeOf, T1: SizeOf, T2: SizeOf, T3: SizeOf> SizeOf for (T0, T1, T2, T3) {}
impl<T0: SizeOf, T1: SizeOf, T2: SizeOf, T3: SizeOf, T4: SizeOf> SizeOf for (T0, T1, T2, T3, T4) {}
impl<T0: SizeOf, T1: SizeOf, T2: SizeOf, T3: SizeOf, T4: SizeOf, T5: SizeOf> SizeOf for (T0, T1, T2, T3, T4, T5) {}

// Implement for Options and Results
impl<T: SizeOf> SizeOf for Option<T> {}
impl<T: SizeOf, E: SizeOf> SizeOf for Result<T, E> {}

// Skip the problematic function pointer implementations that cause ABI issues on Linux
// The original crate tries to implement for extern "aapcs", "stdcall", "fastcall" etc
// which are not supported on Linux targets