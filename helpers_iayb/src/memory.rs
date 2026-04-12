use crate::error::SimpleError;
use asr::signature::Signature;
use asr::{Address, Process};
use std::error::Error;

pub trait Scannable {
    fn scan_range(
        &self,
        process: &Process,
        range: (impl Into<Address>, u64),
    ) -> Result<Address, Box<dyn Error>>;
}

impl<const N: usize> Scannable for &Signature<N> {
    fn scan_range(
        &self,
        process: &Process,
        range: (impl Into<Address>, u64),
    ) -> Result<Address, Box<dyn Error>> {
        self.scan_process_range(process, range)
            .ok_or(SimpleError::from(&format!("unable to find signature {:?}", self)).into())
    }
}

pub fn scan_rel(
    signature: impl Scannable,
    process: &Process,
    module: &str,
    offset: u32,
    next_instruction: u32,
) -> Result<Address, Box<dyn Error>> {
    let module_range = process
        .get_module_range(module)
        .map_err(|_| SimpleError::from(&format!("failed to get range of module {}", module)))?;

    let addr = signature.scan_range(process, module_range)? + offset;

    Ok(addr
        + process
            .read::<i32>(addr)
            .map_err(|_| SimpleError::from(&format!("unable to read from address 0x{}", addr)))?
        + next_instruction)
}
