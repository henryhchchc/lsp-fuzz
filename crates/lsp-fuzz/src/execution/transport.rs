use std::mem;

use libafl_bolts::{ToSliceMut, fs::InputFile, shmem::ShMem};

use crate::libafl_support::AflContext;

/// Describes how the fuzz input is sent to the target.
#[derive(Debug)]
pub enum FuzzInput<SHM> {
    /// Send the input to the target via stdin.
    Stdin(InputFile),
    /// Send the input to the target via a file as an argument.
    File(InputFile),
    /// Send the input to the target via shared memory.
    SharedMemory(SHM),
}

impl<SHM: ShMem> FuzzInput<SHM> {
    /// Sends a serialized fuzz input to the target using the configured transport.
    ///
    /// # Errors
    ///
    /// Returns an error if the input cannot be written to the underlying file or shared memory.
    pub fn send(&mut self, input_bytes: &[u8]) -> Result<(), libafl::Error> {
        match self {
            FuzzInput::Stdin(file) | FuzzInput::File(file) => file.write_buf(input_bytes),
            FuzzInput::SharedMemory(shmem) => Self::write_afl_shmem_input(shmem, input_bytes),
        }
    }

    const SHM_FUZZ_HEADER_SIZE: usize = mem::size_of::<u32>();

    fn write_afl_shmem_input(shmem: &mut SHM, input_bytes: &[u8]) -> Result<(), libafl::Error> {
        use core::sync::atomic::{Ordering, compiler_fence};

        if shmem.len() < input_bytes.len() + Self::SHM_FUZZ_HEADER_SIZE {
            Err(libafl::Error::unknown(
                "The shared memory is too small for the input.",
            ))?;
        }
        let input_size = u32::try_from(input_bytes.len())
            .afl_context("The length of input bytes cannot fit into u32")?;
        let input_size_encoded = input_size.to_ne_bytes();

        compiler_fence(Ordering::Acquire);
        let shmem_slice = shmem.to_slice_mut();
        shmem_slice[..Self::SHM_FUZZ_HEADER_SIZE].copy_from_slice(&input_size_encoded);
        let input_body_range =
            Self::SHM_FUZZ_HEADER_SIZE..(Self::SHM_FUZZ_HEADER_SIZE + input_bytes.len());
        shmem_slice[input_body_range].copy_from_slice(input_bytes);
        compiler_fence(Ordering::Release);

        Ok(())
    }
}
