mod params;
mod perf;

pub use params::*;
pub use perf::Perf;

use crate::{
    batch::Batch,
    context::ContextError::{
        DecodeAborted, DecodeCouldNotFindKvSlot, DecodeFatal, DecodeInvalidInputBatch,
        DecodeUnknown, EncodeUnknown,
    },
};
use snafu::prelude::*;
use std::{
    fmt::{Debug, Formatter},
    num::{NonZeroU8, TryFromIntError},
    ptr::NonNull,
};

/// Safe wrapper around `llama_context`.
pub struct Context {
    raw: NonNull<llama_cpp_sys::llama_context>,
    initialized_logits: Vec<i32>,
    embeddings_enabled: bool,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ContextError {
    #[snafu(display("An unknown error occurred when decode: {code}"))]
    DecodeUnknown { code: i32 },
    #[snafu(display("A fatal error when decode: {code}"))]
    DecodeFatal { code: i32 },
    #[snafu(display("Invalid input batch when decode"))]
    DecodeInvalidInputBatch,
    #[snafu(display("Aborted when decode"))]
    DecodeAborted,
    #[snafu(display(
        "could not find a KV slot for the batch (try reducing the size of the batch or increase the context)"
    ))]
    DecodeCouldNotFindKvSlot,
    #[snafu(display("An unknown error occurred when encode: {code}"))]
    EncodeUnknown { code: i32 },
    #[snafu(display("Provided start position is too large for u32, when memory seq cp"))]
    MemorySeqCpP0TooLarge { source: TryFromIntError },
    #[snafu(display("Provided end position is too large for u32, when memory seq cp"))]
    MemorySeqCpP1TooLarge { source: TryFromIntError },
    #[snafu(display("Provided sequence id is too large for u32, when memory seq cp"))]
    MemorySeqRmIdTooLarge { source: TryFromIntError },
    #[snafu(display("Provided start position is too large for u32, when memory seq rm"))]
    MemorySeqRmP0TooLarge { source: TryFromIntError },
    #[snafu(display("Provided end position is too large for u32, when memory seq rm"))]
    MemorySeqRmP1TooLarge { source: TryFromIntError },
    #[snafu(display("Provided start position is too large for u32, when memory seq add"))]
    MemorySeqAddP0TooLarge { source: TryFromIntError },
    #[snafu(display("Provided end position is too large for u32, when memory seq add"))]
    MemorySeqAddP1TooLarge { source: TryFromIntError },
    #[snafu(display("Provided start position is too large for u32, when memory seq div"))]
    MemorySeqDivP0TooLarge { source: TryFromIntError },
    #[snafu(display("Provided end position is too large for u32, when memory seq div"))]
    MemorySeqDivP1TooLarge { source: TryFromIntError },
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

    /// 处理一批令牌，使用解码器处理批处理
    ///
    /// 正返回值并不意味着致命错误，而是一个警告
    ///
    /// 在发生致命错误或中止时，设法处理的ubatch将保留在上下文的内存状态中，要正确处理此问题，请使用 `llama_memory_seq_pos_min` 和 `llama_memory_seq_pos_max`
    ///
    /// 对于其他返回值，内存状态将恢复到此调用之前的状态
    ///
    /// - 0 - 成功
    /// - 1 - 找不到批次的KV插槽（尝试减少批次的大小或增加上下文）
    /// - 2 - 中止（已处理的ubatch将保留在上下文的内存中）
    /// - -1 - 无效的输入批
    /// - < -1 - 致命错误（处理过的ubatch将保留在上下文的内存中）
    pub fn decode(&mut self, batch: &mut Batch) -> Result<(), ContextError> {
        let result = unsafe { llama_cpp_sys::llama_decode(self.raw.as_ptr(), batch.raw()) };

        match result {
            0 => Ok(()),
            1 => Err(DecodeCouldNotFindKvSlot),
            2 => Err(DecodeAborted),
            -1 => Err(DecodeInvalidInputBatch),
            i if i < -1 => Err(DecodeFatal { code: i }),
            i => Err(DecodeUnknown { code: i }),
        }
    }

    /// 将 `token` 进行编码，不使用 `KV cache`
    ///
    /// 可以在内部存储编码器输出，供 `decoder's cross-attention layers` 使用
    ///
    /// - 0 - 成功
    /// - < 0 - 失败, 内存状态恢复到调用之前的状态
    pub fn encode(&mut self, batch: &mut Batch) -> Result<(), ContextError> {
        let result = unsafe { llama_cpp_sys::llama_encode(self.raw.as_ptr(), batch.raw()) };

        match result {
            0 => Ok(()),
            i => Err(EncodeUnknown { code: i }),
        }
    }

    pub fn copy_cache(&mut self, src: i32, dest: i32, size: i32) {
        unsafe { llama_cpp_sys::llama_memory_seq_cp(self.memory_ptr(), src, dest, 0, size) }
    }

    /// 将属于指定序列的所有令牌复制到另一个序列
    ///
    /// p0 < 0 : [0,  p1]
    /// p1 < 0 : [p0, inf)
    pub fn copy_kv_cache_seq(
        &mut self,
        src: i32,
        dest: i32,
        p0: Option<u32>,
        p1: Option<u32>,
    ) -> Result<(), ContextError> {
        let p0 = p0
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqCpP0TooLargeSnafu)?;
        let p1 = p1
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqCpP1TooLargeSnafu)?;
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
    ) -> Result<bool, ContextError> {
        let src = src
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqRmIdTooLargeSnafu)?;
        let p0 = p0
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqRmP0TooLargeSnafu)?;
        let p1 = p1
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqRmP1TooLargeSnafu)?;
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
    ) -> Result<(), ContextError> {
        let p0 = p0
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqAddP0TooLargeSnafu)?;
        let p1 = p1
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqAddP1TooLargeSnafu)?;
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
    ) -> Result<(), ContextError> {
        let p0 = p0
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqDivP0TooLargeSnafu)?;
        let p1 = p1
            .map_or(Ok(-1), i32::try_from)
            .context(MemorySeqDivP1TooLargeSnafu)?;
        let d = d.get();
        unsafe { llama_cpp_sys::llama_memory_seq_div(self.memory_ptr(), seq_id, p0, p1, d.into()) }
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
