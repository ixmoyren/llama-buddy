//! A safe wrapper around `llama_context_params`
use std::{fmt::Debug, num::NonZeroU32};

/// `llama_rope_scaling_type` 包装器
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RopeScalingType {
    Unspecified = llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_UNSPECIFIED,
    None = llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_NONE,
    Linear = llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_LINEAR,
    Yarn = llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_YARN,
    Longrope = llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_LONGROPE,
}

impl From<i32> for RopeScalingType {
    fn from(value: i32) -> Self {
        use RopeScalingType::*;
        match value {
            llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_NONE => None,
            llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_LINEAR => Linear,
            llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_YARN => Yarn,
            llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_LONGROPE => Longrope,
            llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_UNSPECIFIED => Unspecified,
            _ => panic!("Invalid number for RopeScalingType"),
        }
    }
}

impl From<RopeScalingType> for i32 {
    fn from(value: RopeScalingType) -> Self {
        value as _
    }
}

/// `llama_pooling_type` 包装器
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PoolingType {
    Unspecified = llama_cpp_sys::LLAMA_POOLING_TYPE_UNSPECIFIED,
    None = llama_cpp_sys::LLAMA_POOLING_TYPE_NONE,
    Mean = llama_cpp_sys::LLAMA_POOLING_TYPE_MEAN,
    Cls = llama_cpp_sys::LLAMA_POOLING_TYPE_CLS,
    Last = llama_cpp_sys::LLAMA_POOLING_TYPE_LAST,
    /// 重排序模型用于将分类头附加到图上
    Rank = llama_cpp_sys::LLAMA_POOLING_TYPE_RANK,
}

impl From<i32> for PoolingType {
    fn from(value: i32) -> Self {
        use PoolingType::*;
        match value {
            llama_cpp_sys::LLAMA_POOLING_TYPE_NONE => None,
            llama_cpp_sys::LLAMA_POOLING_TYPE_MEAN => Mean,
            llama_cpp_sys::LLAMA_POOLING_TYPE_CLS => Cls,
            llama_cpp_sys::LLAMA_POOLING_TYPE_LAST => Last,
            llama_cpp_sys::LLAMA_POOLING_TYPE_RANK => Rank,
            llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_UNSPECIFIED => Unspecified,
            _ => panic!("Invalid number for PoolingType"),
        }
    }
}

impl From<PoolingType> for i32 {
    fn from(value: PoolingType) -> Self {
        value as _
    }
}

/// `llama_attention_type` 包装器
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttentionType {
    Unspecified = llama_cpp_sys::LLAMA_ATTENTION_TYPE_UNSPECIFIED,
    Causal = llama_cpp_sys::LLAMA_ATTENTION_TYPE_CAUSAL,
    NonCausal = llama_cpp_sys::LLAMA_ATTENTION_TYPE_NON_CAUSAL,
}

impl From<i32> for AttentionType {
    fn from(value: i32) -> Self {
        use AttentionType::*;
        match value {
            llama_cpp_sys::LLAMA_ATTENTION_TYPE_CAUSAL => Causal,
            llama_cpp_sys::LLAMA_ATTENTION_TYPE_NON_CAUSAL => NonCausal,
            llama_cpp_sys::LLAMA_ROPE_SCALING_TYPE_UNSPECIFIED => Unspecified,
            _ => panic!("Invalid number for AttentionType"),
        }
    }
}

impl From<AttentionType> for i32 {
    fn from(value: AttentionType) -> Self {
        value as _
    }
}

/// `llama_attention_type` 包装器
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FlashAttnType {
    Auto = llama_cpp_sys::LLAMA_FLASH_ATTN_TYPE_AUTO,
    Disabled = llama_cpp_sys::LLAMA_FLASH_ATTN_TYPE_DISABLED,
    Enabled = llama_cpp_sys::LLAMA_FLASH_ATTN_TYPE_ENABLED,
}

impl From<i32> for FlashAttnType {
    fn from(value: i32) -> Self {
        use FlashAttnType::*;
        match value {
            llama_cpp_sys::LLAMA_FLASH_ATTN_TYPE_DISABLED => Disabled,
            llama_cpp_sys::LLAMA_FLASH_ATTN_TYPE_ENABLED => Enabled,
            _ => Auto,
        }
    }
}

impl From<FlashAttnType> for i32 {
    fn from(value: FlashAttnType) -> Self {
        value as _
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GgmlType {
    F32 = llama_cpp_sys::GGML_TYPE_F32,
    F16 = llama_cpp_sys::GGML_TYPE_F16,
    Q40 = llama_cpp_sys::GGML_TYPE_Q4_0,
    Q41 = llama_cpp_sys::GGML_TYPE_Q4_1,
    Q50 = llama_cpp_sys::GGML_TYPE_Q5_0,
    Q51 = llama_cpp_sys::GGML_TYPE_Q5_1,
    Q80 = llama_cpp_sys::GGML_TYPE_Q8_0,
    Q81 = llama_cpp_sys::GGML_TYPE_Q8_1,
    Q2K = llama_cpp_sys::GGML_TYPE_Q2_K,
    Q3K = llama_cpp_sys::GGML_TYPE_Q3_K,
    Q4K = llama_cpp_sys::GGML_TYPE_Q4_K,
    Q5K = llama_cpp_sys::GGML_TYPE_Q5_K,
    Q6K = llama_cpp_sys::GGML_TYPE_Q6_K,
    Q8K = llama_cpp_sys::GGML_TYPE_Q8_K,
    IQ2XXS = llama_cpp_sys::GGML_TYPE_IQ2_XXS,
    IQ2XS = llama_cpp_sys::GGML_TYPE_IQ2_XS,
    IQ3XXS = llama_cpp_sys::GGML_TYPE_IQ3_XXS,
    IQ1S = llama_cpp_sys::GGML_TYPE_IQ1_S,
    IQ4NL = llama_cpp_sys::GGML_TYPE_IQ4_NL,
    IQ3S = llama_cpp_sys::GGML_TYPE_IQ3_S,
    IQ2S = llama_cpp_sys::GGML_TYPE_IQ2_S,
    IQ4XS = llama_cpp_sys::GGML_TYPE_IQ4_XS,
    I8 = llama_cpp_sys::GGML_TYPE_I8,
    I16 = llama_cpp_sys::GGML_TYPE_I16,
    I32 = llama_cpp_sys::GGML_TYPE_I32,
    I64 = llama_cpp_sys::GGML_TYPE_I64,
    F64 = llama_cpp_sys::GGML_TYPE_F64,
    IQ1M = llama_cpp_sys::GGML_TYPE_IQ1_M,
    BF16 = llama_cpp_sys::GGML_TYPE_BF16,
    TQ1_0 = llama_cpp_sys::GGML_TYPE_TQ1_0,
    TQ2_0 = llama_cpp_sys::GGML_TYPE_TQ2_0,
    MXFP4 = llama_cpp_sys::GGML_TYPE_MXFP4,
    COUNT = llama_cpp_sys::GGML_TYPE_COUNT,
}

impl From<u32> for GgmlType {
    fn from(value: u32) -> Self {
        use GgmlType::*;
        match value {
            llama_cpp_sys::GGML_TYPE_F32 => F32,
            llama_cpp_sys::GGML_TYPE_F16 => F16,
            llama_cpp_sys::GGML_TYPE_Q4_0 => Q40,
            llama_cpp_sys::GGML_TYPE_Q4_1 => Q41,
            llama_cpp_sys::GGML_TYPE_Q5_0 => Q50,
            llama_cpp_sys::GGML_TYPE_Q5_1 => Q51,
            llama_cpp_sys::GGML_TYPE_Q8_0 => Q80,
            llama_cpp_sys::GGML_TYPE_Q8_1 => Q81,
            llama_cpp_sys::GGML_TYPE_Q2_K => Q2K,
            llama_cpp_sys::GGML_TYPE_Q3_K => Q3K,
            llama_cpp_sys::GGML_TYPE_Q4_K => Q4K,
            llama_cpp_sys::GGML_TYPE_Q5_K => Q5K,
            llama_cpp_sys::GGML_TYPE_Q6_K => Q6K,
            llama_cpp_sys::GGML_TYPE_Q8_K => Q8K,
            llama_cpp_sys::GGML_TYPE_IQ2_XXS => IQ2XXS,
            llama_cpp_sys::GGML_TYPE_IQ2_XS => IQ2XS,
            llama_cpp_sys::GGML_TYPE_IQ3_XXS => IQ3XXS,
            llama_cpp_sys::GGML_TYPE_IQ1_S => IQ1S,
            llama_cpp_sys::GGML_TYPE_IQ4_NL => IQ4NL,
            llama_cpp_sys::GGML_TYPE_IQ3_S => IQ3S,
            llama_cpp_sys::GGML_TYPE_IQ2_S => IQ2S,
            llama_cpp_sys::GGML_TYPE_IQ4_XS => IQ4XS,
            llama_cpp_sys::GGML_TYPE_I8 => I8,
            llama_cpp_sys::GGML_TYPE_I16 => I16,
            llama_cpp_sys::GGML_TYPE_I32 => I32,
            llama_cpp_sys::GGML_TYPE_I64 => I64,
            llama_cpp_sys::GGML_TYPE_F64 => F64,
            llama_cpp_sys::GGML_TYPE_IQ1_M => IQ1M,
            llama_cpp_sys::GGML_TYPE_BF16 => BF16,
            llama_cpp_sys::GGML_TYPE_TQ1_0 => TQ1_0,
            llama_cpp_sys::GGML_TYPE_TQ2_0 => TQ2_0,
            llama_cpp_sys::GGML_TYPE_MXFP4 => MXFP4,
            llama_cpp_sys::GGML_TYPE_COUNT => COUNT,
            _ => panic!("Invalid number for GgmlType"),
        }
    }
}

impl From<GgmlType> for u32 {
    fn from(value: GgmlType) -> Self {
        value as _
    }
}

#[derive(Debug, Clone)]
pub struct ContextParams {
    raw: llama_cpp_sys::llama_context_params,
}

unsafe impl Send for ContextParams {}
unsafe impl Sync for ContextParams {}

impl ContextParams {
    pub(crate) fn raw(&self) -> llama_cpp_sys::llama_context_params {
        self.raw
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
    pub fn n_seq_max(&self) -> u32 {
        self.raw.n_seq_max
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
    pub fn rope_scaling_type(&self) -> RopeScalingType {
        RopeScalingType::from(self.raw.rope_scaling_type)
    }

    #[must_use]
    pub fn pooling_type(&self) -> PoolingType {
        PoolingType::from(self.raw.pooling_type)
    }

    #[must_use]
    pub fn attention_type(&self) -> AttentionType {
        AttentionType::from(self.raw.attention_type)
    }

    #[must_use]
    pub fn flash_attn_type(&self) -> FlashAttnType {
        FlashAttnType::from(self.raw.flash_attn_type)
    }

    #[must_use]
    pub fn rope_freq_scale(&self) -> f32 {
        self.raw.rope_freq_scale
    }

    #[must_use]
    pub fn rope_freq_base(&self) -> f32 {
        self.raw.rope_freq_base
    }

    #[must_use]
    pub fn type_k(&self) -> GgmlType {
        GgmlType::from(self.raw.type_k)
    }

    #[must_use]
    pub fn type_v(&self) -> GgmlType {
        GgmlType::from(self.raw.type_v)
    }

    #[must_use]
    pub fn embeddings(&self) -> bool {
        self.raw.embeddings
    }

    #[must_use]
    pub fn offload_kqv(&self) -> bool {
        self.raw.offload_kqv
    }

    #[must_use]
    pub fn no_perf(&self) -> bool {
        self.raw.no_perf
    }

    #[must_use]
    pub fn op_offload(&self) -> bool {
        self.raw.op_offload
    }

    #[must_use]
    pub fn swa_full(&self) -> bool {
        self.raw.swa_full
    }

    #[must_use]
    pub fn kv_unified(&self) -> bool {
        self.raw.kv_unified
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
    pub fn with_n_seq_max(mut self, n_seq_max: u32) -> Self {
        self.raw.n_seq_max = n_seq_max;
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
    pub fn with_rope_scaling_type(mut self, rope_scaling_type: RopeScalingType) -> Self {
        self.raw.rope_scaling_type = rope_scaling_type as _;
        self
    }

    #[must_use]
    pub fn with_pooling_type(mut self, pooling_type: PoolingType) -> Self {
        self.raw.pooling_type = pooling_type as _;
        self
    }

    #[must_use]
    pub fn with_attention_type(mut self, attention_type: AttentionType) -> Self {
        self.raw.attention_type = attention_type as _;
        self
    }

    #[must_use]
    pub fn with_flash_attn_type(mut self, flash_attn_type: FlashAttnType) -> Self {
        self.raw.flash_attn_type = flash_attn_type as _;
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
    pub fn with_type_k(mut self, type_k: GgmlType) -> Self {
        self.raw.type_k = type_k as _;
        self
    }

    #[must_use]
    pub fn with_type_v(mut self, type_v: GgmlType) -> Self {
        self.raw.type_v = type_v as _;
        self
    }

    #[must_use]
    pub fn with_embeddings(mut self, embedding: bool) -> Self {
        self.raw.embeddings = embedding;
        self
    }

    #[must_use]
    pub fn with_offload_kqv(mut self, enabled: bool) -> Self {
        self.raw.offload_kqv = enabled;
        self
    }

    /// 用于设置是否启用性能计数器
    /// true - 关闭；false - 开启
    #[must_use]
    pub fn with_no_perf(mut self, no_perf: bool) -> Self {
        self.raw.no_perf = no_perf;
        self
    }

    #[must_use]
    pub fn with_op_offload(mut self, op_offload: bool) -> Self {
        self.raw.op_offload = op_offload;
        self
    }

    #[must_use]
    pub fn with_swa_full(mut self, swa_full: bool) -> Self {
        self.raw.swa_full = swa_full;
        self
    }

    #[must_use]
    pub fn with_kv_unified(mut self, kv_unified: bool) -> Self {
        self.raw.kv_unified = kv_unified;
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
