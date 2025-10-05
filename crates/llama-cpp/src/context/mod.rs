mod params;
mod perf;

pub use params::*;
pub use perf::Perf;

use crate::{
    batch::Batch,
    error::{DecodeError, EncodeError, KvCacheConversionError},
};
use std::{
    ffi::c_int,
    fmt::{Debug, Formatter},
    num::{NonZeroI32, NonZeroU8},
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
            None => Ok(()),
            Some(error_code) => Err(error_code.into()),
        }
    }

    pub fn encode(&mut self, batch: &mut Batch) -> Result<(), EncodeError> {
        let result = unsafe { llama_cpp_sys::llama_encode(self.raw.as_ptr(), batch.raw()) };

        match NonZeroI32::new(result) {
            None => Ok(()),
            Some(error_code) => Err(error_code.into()),
        }
    }

    pub fn copy_cache(&mut self, src: i32, dest: i32, size: i32) {
        unsafe { llama_cpp_sys::llama_memory_seq_cp(self.memory_ptr(), src, dest, 0, size) }
    }

    pub fn copy_kv_cache_seq(
        &mut self,
        src: i32,
        dest: i32,
        p0: Option<u32>,
        p1: Option<u32>,
    ) -> Result<(), KvCacheConversionError> {
        let p0 = p0
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::P0TooLarge)?;
        let p1 = p1
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::P1TooLarge)?;
        unsafe {
            llama_cpp_sys::llama_memory_seq_cp(self.memory_ptr(), src, dest, p0, p1);
        }
        Ok(())
    }

    pub fn clear_kv_cache_seq(
        &mut self,
        src: Option<u32>,
        p0: Option<u32>,
        p1: Option<u32>,
    ) -> Result<bool, KvCacheConversionError> {
        let src = src
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::SeqIdTooLarge)?;
        let p0 = p0
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::P0TooLarge)?;
        let p1 = p1
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::P1TooLarge)?;
        Ok(unsafe { llama_cpp_sys::llama_memory_seq_rm(self.memory_ptr(), src, p0, p1) })
    }

    pub fn clear_kv_cache(&mut self, clear_buf: bool) {
        unsafe { llama_cpp_sys::llama_memory_clear(self.memory_ptr(), clear_buf) }
    }

    pub fn kv_cache_seq_keep(&mut self, seq_id: i32) {
        unsafe { llama_cpp_sys::llama_memory_seq_keep(self.memory_ptr(), seq_id) }
    }

    pub fn kv_cache_seq_add(
        &mut self,
        seq_id: i32,
        p0: Option<u32>,
        p1: Option<u32>,
        delta: i32,
    ) -> Result<(), KvCacheConversionError> {
        let p0 = p0
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::P0TooLarge)?;
        let p1 = p1
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::P1TooLarge)?;
        unsafe {
            llama_cpp_sys::llama_memory_seq_add(self.memory_ptr(), seq_id, p0, p1, delta);
        }
        Ok(())
    }

    pub fn kv_cache_seq_div(
        &mut self,
        seq_id: i32,
        p0: Option<u32>,
        p1: Option<u32>,
        d: NonZeroU8,
    ) -> Result<(), KvCacheConversionError> {
        let p0 = p0
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::P0TooLarge)?;
        let p1 = p1
            .map_or(Ok(-1), i32::try_from)
            .map_err(KvCacheConversionError::P1TooLarge)?;
        let d = c_int::from(d.get());
        unsafe { llama_cpp_sys::llama_memory_seq_div(self.memory_ptr(), seq_id, p0, p1, d) }
        Ok(())
    }

    #[must_use]
    pub fn kv_cache_seq_pos_max(&self, seq_id: i32) -> i32 {
        unsafe { llama_cpp_sys::llama_memory_seq_pos_max(self.memory_ptr(), seq_id) }
    }

    #[must_use]
    pub fn kv_cache_seq_pos_min(&self, seq_id: i32) -> i32 {
        unsafe { llama_cpp_sys::llama_memory_seq_pos_min(self.memory_ptr(), seq_id) }
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
