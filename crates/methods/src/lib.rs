// methods/src/lib.rs

#[cfg(not(guest_code_not_built))]
include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(guest_code_not_built)]
mod methods_placeholder {
    pub const MMR_GUEST_ELF: &[u8] = &[];
    pub const MMR_GUEST_ID: [u32; 8] = [0; 8];
}

#[cfg(guest_code_not_built)]
pub use methods_placeholder::*;
