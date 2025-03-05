use crate::error::GgmlNumaStrategyError;

/// `ggml_numa_strategy` 的包装器
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Strategy {
    DISABLED,
    DISTRIBUTE,
    ISOLATE,
    NUMACTL,
    MIRROR,
    COUNT,
}

impl TryFrom<llama_cpp_sys::ggml_numa_strategy> for Strategy {
    type Error = GgmlNumaStrategyError;

    fn try_from(value: llama_cpp_sys::ggml_numa_strategy) -> Result<Self, Self::Error> {
        use Strategy::*;
        match value {
            llama_cpp_sys::GGML_NUMA_STRATEGY_DISABLED => Ok(DISABLED),
            llama_cpp_sys::GGML_NUMA_STRATEGY_DISTRIBUTE => Ok(DISTRIBUTE),
            llama_cpp_sys::GGML_NUMA_STRATEGY_ISOLATE => Ok(ISOLATE),
            llama_cpp_sys::GGML_NUMA_STRATEGY_NUMACTL => Ok(NUMACTL),
            llama_cpp_sys::GGML_NUMA_STRATEGY_MIRROR => Ok(MIRROR),
            llama_cpp_sys::GGML_NUMA_STRATEGY_COUNT => Ok(COUNT),
            strategy => Err(GgmlNumaStrategyError::NonsupportValue(strategy.to_string())),
        }
    }
}

impl From<Strategy> for llama_cpp_sys::ggml_numa_strategy {
    fn from(value: Strategy) -> Self {
        use Strategy::*;
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
