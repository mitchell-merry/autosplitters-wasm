use crate::error::SimpleError;
use asr::signature::Signature;
use asr::{Address, Process};
use std::error::Error;

pub fn scan_rel<const N: usize>(
    signature: &Signature<N>,
    process: &Process,
    module: &str,
    offset: u32,
    next_instruction: u32,
) -> Result<Address, Box<dyn Error>> {
    let module_range = process
        .get_module_range(module)
        .map_err(|_| SimpleError::from(&format!("failed to get range of module {}", module)))?;

    let addr = signature
        .scan_process_range(process, module_range)
        .ok_or(SimpleError::from(&format!(
            "unable to find signature in module {}",
            module
        )))?
        + offset;

    Ok(addr
        + process
            .read::<u32>(addr)
            .map_err(|_| SimpleError::from(&format!("unable to read from address 0x{}", addr)))?
        + next_instruction)
}
