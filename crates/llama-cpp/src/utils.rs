pub fn ggml_time_init() {
    unsafe { llama_cpp_sys::ggml_time_init() }
}

pub fn ggml_time_ms() -> i64 {
    unsafe { llama_cpp_sys::ggml_time_ms() }
}

pub fn ggml_time_us() -> i64 {
    unsafe { llama_cpp_sys::ggml_time_us() }
}
