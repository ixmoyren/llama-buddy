//! A safe wrapper around `llama_context_params`.
use std::{fmt::Debug, num::NonZeroU32};

/// `rope_scaling_type` 包装器
#[repr(i8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RopeScalingType {
    Unspecified = -1,
    None = 0,
    Linear = 1,
    Yarn = 2,
}

impl From<i32> for RopeScalingType {
    fn from(value: i32) -> Self {
        use RopeScalingType::*;
        match value {
            0 => None,
            1 => Linear,
            2 => Yarn,
            _ => Unspecified,
        }
    }
}

impl From<RopeScalingType> for i32 {
    fn from(value: RopeScalingType) -> Self {
        use RopeScalingType::*;
        match value {
            None => 0,
            Linear => 1,
            Yarn => 2,
            Unspecified => -1,
        }
    }
}

/// `LLAMA_POOLING_TYPE` 包装器
#[repr(i8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PoolingType {
    Unspecified = -1,
    None = 0,
    Mean = 1,
    Cls = 2,
    Last = 3,
    Rank = 4,
}

/// Create a `LlamaPoolingType` from a `c_int` - returns `LlamaPoolingType::Unspecified` if
/// the value is not recognized.
impl From<i32> for PoolingType {
    fn from(value: i32) -> Self {
        use PoolingType::*;
        match value {
            0 => None,
            1 => Mean,
            2 => Cls,
            3 => Last,
            4 => Rank,
            _ => Unspecified,
        }
    }
}

/// Create a `c_int` from a `LlamaPoolingType`.
impl From<PoolingType> for i32 {
    fn from(value: PoolingType) -> Self {
        use PoolingType::*;
        match value {
            None => 0,
            Mean => 1,
            Cls => 2,
            Last => 3,
            Rank => 4,
            Unspecified => -1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextParams {
    raw: llama_cpp_sys::llama_context_params,
}

unsafe impl Send for ContextParams {}
unsafe impl Sync for ContextParams {}

impl ContextParams {
    pub fn raw(&self) -> llama_cpp_sys::llama_context_params {
        self.raw
    }

    #[must_use]
    pub fn flash_attention(&self) -> bool {
        self.raw.flash_attn
    }

    #[must_use]
    pub fn n_ctx(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.raw.n_ctx)
    }

    #[must_use]
    pub fn n_batch(&self) -> u32 {
        self.raw.n_batch
    }

    #[must_use]
    pub fn n_ubatch(&self) -> u32 {
        self.raw.n_ubatch
    }

    #[must_use]
    pub fn offload_kqv(&self) -> bool {
        self.raw.offload_kqv
    }

    #[must_use]
    pub fn rope_scaling_type(&self) -> RopeScalingType {
        RopeScalingType::from(self.raw.rope_scaling_type)
    }

    #[must_use]
    pub fn rope_freq_scale(&self) -> f32 {
        self.raw.rope_freq_scale
    }

    #[must_use]
    pub fn n_threads(&self) -> i32 {
        self.raw.n_threads
    }

    #[must_use]
    pub fn n_threads_batch(&self) -> i32 {
        self.raw.n_threads_batch
    }

    #[must_use]
    pub fn rope_freq_base(&self) -> f32 {
        self.raw.rope_freq_base
    }

    #[must_use]
    pub fn embeddings(&self) -> bool {
        self.raw.embeddings
    }

    #[must_use]
    pub fn pooling_type(&self) -> PoolingType {
        PoolingType::from(self.raw.pooling_type)
    }

    #[must_use]
    pub fn n_perf(&self) -> bool {
        self.raw.no_perf
    }

    /// 设置上下文大小
    #[must_use]
    pub fn with_n_ctx(mut self, n_ctx: u32) -> Self {
        self.raw.n_ctx = n_ctx;
        self
    }

    /// 设置每次调用 llama_decode 处理的最大令牌数
    #[must_use]
    pub fn with_n_batch(mut self, n_batch: u32) -> Self {
        self.raw.n_batch = n_batch;
        self
    }

    #[must_use]
    pub fn with_n_ubatch(mut self, n_ubatch: u32) -> Self {
        self.raw.n_ubatch = n_ubatch;
        self
    }

    #[must_use]
    pub fn with_flash_attention(mut self, enabled: bool) -> Self {
        self.raw.flash_attn = enabled;
        self
    }

    #[must_use]
    pub fn with_offload_kqv(mut self, enabled: bool) -> Self {
        self.raw.offload_kqv = enabled;
        self
    }

    #[must_use]
    pub fn with_rope_scaling_type(mut self, rope_scaling_type: RopeScalingType) -> Self {
        self.raw.rope_scaling_type = i32::from(rope_scaling_type);
        self
    }

    #[must_use]
    pub fn with_rope_freq_base(mut self, rope_freq_base: f32) -> Self {
        self.raw.rope_freq_base = rope_freq_base;
        self
    }

    #[must_use]
    pub fn with_rope_freq_scale(mut self, rope_freq_scale: f32) -> Self {
        self.raw.rope_freq_scale = rope_freq_scale;
        self
    }

    #[must_use]
    pub fn with_n_threads(mut self, n_threads: i32) -> Self {
        self.raw.n_threads = n_threads;
        self
    }

    #[must_use]
    pub fn with_n_threads_batch(mut self, n_threads: i32) -> Self {
        self.raw.n_threads_batch = n_threads;
        self
    }

    #[must_use]
    pub fn with_embeddings(mut self, embedding: bool) -> Self {
        self.raw.embeddings = embedding;
        self
    }

    #[must_use]
    pub fn with_cb_eval(
        mut self,
        cb_eval: llama_cpp_sys::ggml_backend_sched_eval_callback,
    ) -> Self {
        self.raw.cb_eval = cb_eval;
        self
    }

    #[must_use]
    pub fn with_cb_eval_user_data(mut self, cb_eval_user_data: *mut std::ffi::c_void) -> Self {
        self.raw.cb_eval_user_data = cb_eval_user_data;
        self
    }

    #[must_use]
    pub fn with_pooling_type(mut self, pooling_type: PoolingType) -> Self {
        self.raw.pooling_type = i32::from(pooling_type);
        self
    }

    /// 用于设置是否启用性能计数器
    /// true - 关闭；false - 开启
    #[must_use]
    pub fn with_n_perf(mut self, n_perf: bool) -> Self {
        self.raw.no_perf = n_perf;
        self
    }
}

impl Default for ContextParams {
    fn default() -> Self {
        let context_params = unsafe { llama_cpp_sys::llama_context_default_params() };
        Self {
            raw: context_params,
        }
    }
}
