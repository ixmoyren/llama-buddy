//! utilities for working with the kv cache

use super::Context;
use crate::error::KvCacheConversionError;
use std::{ffi::c_int, num::NonZeroU8};

impl Context {
    pub fn copy_cache(&mut self, src: i32, dest: i32, size: i32) {
        unsafe { llama_cpp_sys::llama_kv_self_seq_cp(self.raw_mut(), src, dest, 0, size) }
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
            llama_cpp_sys::llama_kv_self_seq_cp(self.raw_mut(), src, dest, p0, p1);
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
        Ok(unsafe { llama_cpp_sys::llama_kv_self_seq_rm(self.raw_mut(), src, p0, p1) })
    }

    #[must_use]
    pub fn get_kv_cache_used_cells(&self) -> i32 {
        unsafe { llama_cpp_sys::llama_kv_self_used_cells(self.raw_mut()) }
    }

    pub fn clear_kv_cache(&mut self) {
        unsafe { llama_cpp_sys::llama_kv_self_clear(self.raw_mut()) }
    }

    pub fn llama_kv_cache_seq_keep(&mut self, seq_id: i32) {
        unsafe { llama_cpp_sys::llama_kv_self_seq_keep(self.raw_mut(), seq_id) }
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
            llama_cpp_sys::llama_kv_self_seq_add(self.raw_mut(), seq_id, p0, p1, delta);
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
        unsafe { llama_cpp_sys::llama_kv_self_seq_div(self.raw_mut(), seq_id, p0, p1, d) }
        Ok(())
    }

    #[must_use]
    pub fn kv_cache_seq_pos_max(&self, seq_id: i32) -> i32 {
        unsafe { llama_cpp_sys::llama_kv_self_seq_pos_max(self.raw_mut(), seq_id) }
    }

    pub fn kv_cache_defrag(&mut self) {
        unsafe { llama_cpp_sys::llama_kv_self_defrag(self.raw_mut()) }
    }

    /// Apply the KV cache updates (such as K-shifts, defragmentation, etc.)
    pub fn kv_cache_update(&mut self) {
        unsafe { llama_cpp_sys::llama_kv_self_update(self.raw_mut()) }
    }
}
