use crate::error::BackendError;
use llama_cpp_sys::ggml_log_level;
use std::sync::Once;
use tracing::info;

/// 用于初始化 llama.cpp 后端
#[derive(Debug, Eq, PartialEq)]
pub struct Backend {
    ggml_numa_strategy: Option<GgmlNumaStrategy>,
}

static INITIALIZED_ONCE: Once = Once::new();

impl Backend {
    pub fn new() -> Backend {
        Backend {
            ggml_numa_strategy: None,
        }
    }

    #[tracing::instrument(level = "info")]
    pub fn init(&self) {
        INITIALIZED_ONCE.call_once(|| unsafe {
            if let Some(strategy) = self.ggml_numa_strategy {
                let strategy = strategy.into();
                llama_cpp_sys::llama_numa_init(strategy);
                info!("Initialize the llama backend (with numa: {strategy}).")
            } else {
                llama_cpp_sys::llama_backend_init();
                info!("Initialized llama.cpp backend (without numa).");
            }
        })
    }

    /// 是否支持 GPU
    pub fn supports_gpu_offload(&self) -> bool {
        unsafe { llama_cpp_sys::llama_supports_gpu_offload() }
    }

    /// 是否支持 mmap
    pub fn supports_mmap(&self) -> bool {
        unsafe { llama_cpp_sys::llama_supports_mmap() }
    }

    /// 是否支持将模型锁定在内存里面
    pub fn supports_mlock(&self) -> bool {
        unsafe { llama_cpp_sys::llama_supports_mlock() }
    }

    /// 不输出日志
    pub fn void_logs(&mut self) {
        unsafe extern "C" fn void_log(
            _level: ggml_log_level,
            _text: *const std::os::raw::c_char,
            _user_data: *mut std::os::raw::c_void,
        ) {
        }

        unsafe {
            llama_cpp_sys::llama_log_set(Some(void_log), std::ptr::null_mut());
        }
    }
}

impl Drop for Backend {
    #[tracing::instrument(level = "info")]
    fn drop(&mut self) {
        INITIALIZED_ONCE.call_once(|| unsafe {
            llama_cpp_sys::llama_backend_free();
            info!("Freed llama.cpp backend");
        })
    }
}

/// `ggml_numa_strategy` 的包装器
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum GgmlNumaStrategy {
    DISABLED,
    DISTRIBUTE,
    ISOLATE,
    NUMACTL,
    MIRROR,
    COUNT,
}

impl TryFrom<llama_cpp_sys::ggml_numa_strategy> for GgmlNumaStrategy {
    type Error = BackendError;

    fn try_from(value: llama_cpp_sys::ggml_numa_strategy) -> Result<Self, Self::Error> {
        use GgmlNumaStrategy::*;
        match value {
            llama_cpp_sys::GGML_NUMA_STRATEGY_DISABLED => Ok(DISABLED),
            llama_cpp_sys::GGML_NUMA_STRATEGY_DISTRIBUTE => Ok(DISTRIBUTE),
            llama_cpp_sys::GGML_NUMA_STRATEGY_ISOLATE => Ok(ISOLATE),
            llama_cpp_sys::GGML_NUMA_STRATEGY_NUMACTL => Ok(NUMACTL),
            llama_cpp_sys::GGML_NUMA_STRATEGY_MIRROR => Ok(MIRROR),
            llama_cpp_sys::GGML_NUMA_STRATEGY_COUNT => Ok(COUNT),
            value => Err(BackendError::NonsupportGgmlNumaStrategy(value.to_string())),
        }
    }
}

impl From<GgmlNumaStrategy> for llama_cpp_sys::ggml_numa_strategy {
    fn from(value: GgmlNumaStrategy) -> Self {
        use GgmlNumaStrategy::*;
        match value {
            DISABLED => llama_cpp_sys::GGML_NUMA_STRATEGY_DISABLED,
            DISTRIBUTE => llama_cpp_sys::GGML_NUMA_STRATEGY_DISTRIBUTE,
            ISOLATE => llama_cpp_sys::GGML_NUMA_STRATEGY_ISOLATE,
            NUMACTL => llama_cpp_sys::GGML_NUMA_STRATEGY_NUMACTL,
            MIRROR => llama_cpp_sys::GGML_NUMA_STRATEGY_MIRROR,
            COUNT => llama_cpp_sys::GGML_NUMA_STRATEGY_COUNT,
        }
    }
}
