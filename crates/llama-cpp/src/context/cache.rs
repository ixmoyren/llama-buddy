//! utilities for working with the kv cache

use super::Context;
use crate::error::KvCacheConversionError;
use std::{ffi::c_int, num::NonZeroU8};

impl Context {
    pub fn copy_cache(&mut self, src: i32, dest: i32, size: i32) {
        unsafe { llama_cpp_sys::llama_kv_cache_seq_cp(self.raw_mut(), src, dest, 0, size) }
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
            llama_cpp_sys::llama_kv_cache_seq_cp(self.raw_mut(), src, dest, p0, p1);
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
        Ok(unsafe { llama_cpp_sys::llama_kv_cache_seq_rm(self.raw_mut(), src, p0, p1) })
    }

    #[must_use]
    pub fn get_kv_cache_used_cells(&self) -> i32 {
        unsafe { llama_cpp_sys::llama_get_kv_cache_used_cells(self.raw_mut()) }
    }

    pub fn clear_kv_cache(&mut self) {
        unsafe { llama_cpp_sys::llama_kv_cache_clear(self.raw_mut()) }
    }

    pub fn llama_kv_cache_seq_keep(&mut self, seq_id: i32) {
        unsafe { llama_cpp_sys::llama_kv_cache_seq_keep(self.raw_mut(), seq_id) }
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
            llama_cpp_sys::llama_kv_cache_seq_add(self.raw_mut(), seq_id, p0, p1, delta);
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
        unsafe { llama_cpp_sys::llama_kv_cache_seq_div(self.raw_mut(), seq_id, p0, p1, d) }
        Ok(())
    }

    #[must_use]
    pub fn kv_cache_seq_pos_max(&self, seq_id: i32) -> i32 {
        unsafe { llama_cpp_sys::llama_kv_cache_seq_pos_max(self.raw_mut(), seq_id) }
    }

    pub fn kv_cache_defrag(&mut self) {
        unsafe { llama_cpp_sys::llama_kv_cache_defrag(self.raw_mut()) }
    }

    /// Apply the KV cache updates (such as K-shifts, defragmentation, etc.)
    pub fn kv_cache_update(&mut self) {
        unsafe { llama_cpp_sys::llama_kv_cache_update(self.raw_mut()) }
    }

    /// Returns the number of tokens in the KV cache (slow, use only for debug)
    /// If a KV cell has multiple sequences assigned to it, it will be counted multiple times
    #[must_use]
    pub fn get_kv_cache_token_count(&self) -> i32 {
        unsafe { llama_cpp_sys::llama_get_kv_cache_token_count(self.raw_mut()) }
    }

    #[must_use]
    pub fn new_kv_cache_view(&self, n_max_seq: i32) -> KVCacheView {
        let view = unsafe { llama_cpp_sys::llama_kv_cache_view_init(self.raw_mut(), n_max_seq) };
        KVCacheView { view }
    }
}

#[derive(Debug)]
pub struct KVCacheViewCell {
    pub pos: llama_cpp_sys::llama_pos,
}

#[derive(Debug)]
pub struct KVCacheView {
    view: llama_cpp_sys::llama_kv_cache_view,
}

impl KVCacheView {
    /// Update the KV cache view structure with the current state of the KV cache. (use only for debugging purposes)
    pub fn update(&mut self, context: &Context) {
        unsafe {
            llama_cpp_sys::llama_kv_cache_view_update(context.raw_mut(), &mut self.view);
        }
    }

    /// Number of KV cache cells. This will be the same as the context size.
    #[must_use]
    pub fn n_cells(&self) -> i32 {
        self.view.n_cells
    }

    /// Number of tokens in the cache. For example, if there are two populated
    /// cells, the first with 1 sequence id in it and the second with 2 sequence
    /// ids then you'll have 3 tokens.
    #[must_use]
    pub fn token_count(&self) -> i32 {
        self.view.token_count
    }

    /// Number of populated cache cells.
    #[must_use]
    pub fn used_cells(&self) -> i32 {
        self.view.used_cells
    }

    /// Maximum contiguous empty slots in the cache.
    #[must_use]
    pub fn max_contiguous(&self) -> i32 {
        self.view.max_contiguous
    }

    /// Index to the start of the `max_contiguous` slot range. Can be negative
    /// when cache is full.
    #[must_use]
    pub fn max_contiguous_idx(&self) -> i32 {
        self.view.max_contiguous_idx
    }

    /// Information for individual cells.
    ///
    /// # Panics
    ///
    /// - if `n_cells` does not fit into usize.
    pub fn cells(&self) -> impl Iterator<Item = KVCacheViewCell> {
        unsafe {
            std::slice::from_raw_parts(
                self.view.cells,
                usize::try_from(self.view.n_cells).expect("failed to fit n_cells into usize"),
            )
        }
        .iter()
        .map(|&cell| KVCacheViewCell { pos: cell.pos })
    }

    /// The sequences for each cell. There will be `n_max_seq` items per cell.
    ///
    /// # Panics
    ///
    /// - if `n_cells * n_max_seq` does not fit into usize.
    /// - if `n_max_seq` does not fit into usize.
    pub fn cells_sequences(&self) -> impl Iterator<Item = &[llama_cpp_sys::llama_seq_id]> {
        unsafe {
            std::slice::from_raw_parts(
                self.view.cells_sequences,
                usize::try_from(self.view.n_cells * self.view.n_seq_max)
                    .expect("failed to fit n_cells * n_max_seq into usize"),
            )
        }
        .chunks(usize::try_from(self.view.n_seq_max).expect("failed to fit n_max_seq into usize"))
    }
}

impl Drop for KVCacheView {
    fn drop(&mut self) {
        unsafe {
            llama_cpp_sys::llama_kv_cache_view_free(&mut self.view);
        }
    }
}
