use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// `llama_adapter_lora` 的包装
/// 基于 LoRA（Low-Rank Adaptation）技术实现的、专为 LLaMA 系列大语言模型设计的参数高效微调模块
/// 其核心作用是在不修改原模型大部分参数的前提下，通过引入可训练的低秩矩阵对模型行为进行适配调整
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct AdapterLora {
    raw: NonNull<llama_cpp_sys::llama_adapter_lora>,
}

impl AdapterLora {
    pub fn raw_mut(&self) -> *mut llama_cpp_sys::llama_adapter_lora {
        let raw = self.raw.as_ptr();
        raw
    }
}

impl From<NonNull<llama_cpp_sys::llama_adapter_lora>> for AdapterLora {
    fn from(raw: NonNull<llama_cpp_sys::llama_adapter_lora>) -> Self {
        Self { raw }
    }
}

impl Deref for AdapterLora {
    type Target = llama_cpp_sys::llama_adapter_lora;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.raw.as_ref() }
    }
}

impl DerefMut for AdapterLora {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.raw.as_mut() }
    }
}

impl Drop for AdapterLora {
    fn drop(&mut self) {
        unsafe { llama_cpp_sys::llama_adapter_lora_free(self.raw.as_ptr()) }
    }
}
