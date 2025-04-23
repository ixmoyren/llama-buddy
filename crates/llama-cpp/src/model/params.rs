use std::{
    ffi::{c_char, CStr, CString},
    ops::{Deref, DerefMut},
    slice,
};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct ModelParams {
    raw: llama_cpp_sys::llama_model_params,
}

impl Default for ModelParams {
    fn default() -> Self {
        let raw = unsafe { llama_cpp_sys::llama_model_default_params() };
        Self { raw }
    }
}

impl ModelParams {
    pub fn raw(&self) -> llama_cpp_sys::llama_model_params {
        self.raw
    }

    pub fn with_n_gpu_layers(mut self, n: i32) -> Self {
        self.raw.n_gpu_layers = n;
        self
    }

    pub fn with_split_mode(mut self, mode: u32) -> Self {
        self.raw.split_mode = mode as _;
        self
    }

    pub fn with_main_gpu(mut self, gpu: i32) -> Self {
        self.raw.n_gpu_layers = gpu;
        self
    }

    pub fn with_kv_override(mut self, key: KvOverrideKey, value: KvOverrideValue) -> Self {
        let kv_override = Into::<KvOverride>::into((key, value)).raw;
        self.raw.kv_overrides = &kv_override;
        self
    }

    pub fn with_vocab_only(mut self, only: bool) -> Self {
        self.raw.vocab_only = only;
        self
    }

    pub fn with_use_mmap(mut self, switch: bool) -> Self {
        self.raw.use_mmap = switch;
        self
    }

    pub fn with_use_mlock(mut self, switch: bool) -> Self {
        self.raw.use_mlock = switch;
        self
    }

    pub fn with_check_tensors(mut self, check: bool) -> Self {
        self.raw.vocab_only = check;
        self
    }
}

impl Deref for ModelParams {
    type Target = llama_cpp_sys::llama_model_params;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl DerefMut for ModelParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct KvOverride {
    raw: llama_cpp_sys::llama_model_kv_override,
}

impl KvOverride {
    pub fn value(&self) -> KvOverrideValue {
        let llama_cpp_sys::llama_model_kv_override {
            key: _,
            tag,
            __bindgen_anon_1,
        } = &self.raw;
        match *tag {
            llama_cpp_sys::LLAMA_KV_OVERRIDE_TYPE_INT => {
                KvOverrideValue::Int(unsafe { __bindgen_anon_1.val_i64 })
            }
            llama_cpp_sys::LLAMA_KV_OVERRIDE_TYPE_FLOAT => {
                KvOverrideValue::Float(unsafe { __bindgen_anon_1.val_f64 })
            }
            llama_cpp_sys::LLAMA_KV_OVERRIDE_TYPE_BOOL => {
                KvOverrideValue::Bool(unsafe { __bindgen_anon_1.val_bool })
            }
            llama_cpp_sys::LLAMA_KV_OVERRIDE_TYPE_STR => {
                let bytes = unsafe {
                    slice::from_raw_parts(
                        __bindgen_anon_1.val_str.as_ptr() as *const u8,
                        __bindgen_anon_1.val_str.len(),
                    )
                };
                let str = CStr::from_bytes_with_nul(bytes)
                    .expect("invalid string from llama kv override value str");
                let str = str
                    .to_str()
                    .expect("llama kv override value str can not covert to utf-8 str")
                    .to_owned();
                KvOverrideValue::Str(str)
            }
            _ => unreachable!("Unknown tag of {tag}"),
        }
    }

    pub fn key(&self) -> KvOverrideKey {
        let llama_cpp_sys::llama_model_kv_override {
            key,
            tag: _,
            __bindgen_anon_1: _,
        } = &self.raw;
        let key = unsafe { CStr::from_ptr(key.as_ptr()) };
        let key = key
            .to_str()
            .expect("llama kv override str can not covert to utf-8 str")
            .to_owned();
        KvOverrideKey(key)
    }
}

impl From<(KvOverrideKey, KvOverrideValue)> for KvOverride {
    fn from((key, value): (KvOverrideKey, KvOverrideValue)) -> Self {
        let key = key.original_value();
        let tag = value.original_type();
        let value = value.original_value();
        let raw = llama_cpp_sys::llama_model_kv_override {
            key,
            tag,
            __bindgen_anon_1: value,
        };
        Self { raw }
    }
}

pub struct KvOverrideKey(String);

impl KvOverrideKey {
    pub(crate) fn original_value(&self) -> [c_char; 128] {
        let c_string =
            CString::new(self.0.as_bytes()).expect("nul byte in string by kv_override_key");
        let bytes_with_nul = c_string.as_bytes_with_nul();
        let mut val_str: [c_char; 128] = [0; 128];
        for (i, &byte) in bytes_with_nul.iter().enumerate() {
            val_str[i] = byte as c_char;
        }
        val_str
    }
}

impl Deref for KvOverrideKey {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KvOverrideKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KvOverrideValue {
    Bool(bool),
    Float(f64),
    Int(i64),
    Str(String),
}

impl KvOverrideValue {
    pub(crate) fn original_type(&self) -> llama_cpp_sys::llama_model_kv_override_type {
        match self {
            KvOverrideValue::Bool(_) => llama_cpp_sys::LLAMA_KV_OVERRIDE_TYPE_BOOL,
            KvOverrideValue::Float(_) => llama_cpp_sys::LLAMA_KV_OVERRIDE_TYPE_FLOAT,
            KvOverrideValue::Int(_) => llama_cpp_sys::LLAMA_KV_OVERRIDE_TYPE_INT,
            KvOverrideValue::Str(_) => llama_cpp_sys::LLAMA_KV_OVERRIDE_TYPE_STR,
        }
    }

    pub(crate) fn original_value(&self) -> llama_cpp_sys::llama_model_kv_override__bindgen_ty_1 {
        match self {
            KvOverrideValue::Bool(value) => {
                llama_cpp_sys::llama_model_kv_override__bindgen_ty_1 { val_bool: *value }
            }
            KvOverrideValue::Float(value) => {
                llama_cpp_sys::llama_model_kv_override__bindgen_ty_1 { val_f64: *value }
            }
            KvOverrideValue::Int(value) => {
                llama_cpp_sys::llama_model_kv_override__bindgen_ty_1 { val_i64: *value }
            }
            KvOverrideValue::Str(str) => {
                let str =
                    CString::new(str.as_bytes()).expect("nul byte in string by kv_override_value");
                let bytes_with_nul = str.as_bytes_with_nul();
                let mut val_str: [c_char; 128] = [0; 128];
                for (i, &byte) in bytes_with_nul.iter().enumerate() {
                    val_str[i] = byte as c_char;
                }
                llama_cpp_sys::llama_model_kv_override__bindgen_ty_1 { val_str }
            }
        }
    }
}
