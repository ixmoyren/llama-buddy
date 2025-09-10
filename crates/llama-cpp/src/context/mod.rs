mod cache;
mod params;
mod perf;

pub use cache::*;
pub use params::*;
pub use perf::Perf;

use crate::{
    batch::Batch,
    error::{DecodeError, EncodeError},
};
use std::{
    fmt::{Debug, Formatter},
    num::NonZeroI32,
    ptr::NonNull,
};

/// Safe wrapper around `llama_context`.
pub struct Context {
    raw: NonNull<llama_cpp_sys::llama_context>,
    initialized_logits: Vec<i32>,
    embeddings_enabled: bool,
}

impl Debug for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlamaContext")
            .field("context", &self.raw)
            .finish()
    }
}

impl Context {
    pub fn raw_mut(&self) -> *mut llama_cpp_sys::llama_context {
        self.raw.as_ptr()
    }

    pub fn memory_ptr(&self) -> llama_cpp_sys::llama_memory_t {
        unsafe { llama_cpp_sys::llama_get_memory(self.raw.as_ptr()) }
    }

    pub fn initialized_logits(&self) -> &[i32] {
        self.initialized_logits.as_slice()
    }

    pub fn embeddings_enabled(&self) -> bool {
        self.embeddings_enabled
    }

    pub fn new(
        llama_context: NonNull<llama_cpp_sys::llama_context>,
        embeddings_enabled: bool,
    ) -> Self {
        Self {
            raw: llama_context,
            initialized_logits: Vec::new(),
            embeddings_enabled,
        }
    }

    #[must_use]
    pub fn n_batch(&self) -> u32 {
        unsafe { llama_cpp_sys::llama_n_batch(self.raw.as_ptr()) }
    }

    #[must_use]
    pub fn n_ubatch(&self) -> u32 {
        unsafe { llama_cpp_sys::llama_n_ubatch(self.raw.as_ptr()) }
    }

    #[must_use]
    pub fn n_ctx(&self) -> u32 {
        unsafe { llama_cpp_sys::llama_n_ctx(self.raw.as_ptr()) }
    }

    pub fn decode(&mut self, batch: &mut Batch) -> Result<(), DecodeError> {
        let result = unsafe { llama_cpp_sys::llama_decode(self.raw.as_ptr(), batch.raw()) };

        match NonZeroI32::new(result) {
            None => {
                self.initialized_logits.clone_from_slice(batch.logits());
                Ok(())
            }
            Some(error_code) => Err(error_code.into()),
        }
    }

    pub fn encode(&mut self, batch: &mut Batch) -> Result<(), EncodeError> {
        let result = unsafe { llama_cpp_sys::llama_encode(self.raw.as_ptr(), batch.raw()) };

        match NonZeroI32::new(result) {
            None => {
                self.initialized_logits.clone_from_slice(batch.logits());
                Ok(())
            }
            Some(error_code) => Err(error_code.into()),
        }
    }

    /// Reset the timings for the context.
    pub fn reset_timings(&mut self) {
        unsafe { llama_cpp_sys::llama_perf_context_reset(self.raw.as_ptr()) }
    }

    /// Returns the timings for the context.
    pub fn timings(&mut self) -> Perf {
        let timings = unsafe { llama_cpp_sys::llama_perf_context(self.raw.as_ptr()) };
        timings.into()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { llama_cpp_sys::llama_free(self.raw.as_ptr()) }
    }
}
