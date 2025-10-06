use snafu::prelude::*;
use std::{fmt, fmt::Formatter};

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

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum StrategyError {
    #[snafu(display("Nonsupport ggml numa strategy: {from}"))]
    NonsupportValue {
        from: llama_cpp_sys::ggml_numa_strategy,
    },
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Strategy::DISABLED => {
                write!(f, "GGML_NUMA_STRATEGY_DISABLED")
            }
            Strategy::DISTRIBUTE => {
                write!(f, "GGML_NUMA_STRATEGY_DISTRIBUTE")
            }
            Strategy::ISOLATE => {
                write!(f, "GGML_NUMA_STRATEGY_ISOLATE")
            }
            Strategy::NUMACTL => {
                write!(f, "GGML_NUMA_STRATEGY_NUMACTL")
            }
            Strategy::MIRROR => {
                write!(f, "GGML_NUMA_STRATEGY_MIRROR")
            }
            Strategy::COUNT => {
                write!(f, "GGML_NUMA_STRATEGY_COUNT")
            }
        }
    }
}

impl TryFrom<llama_cpp_sys::ggml_numa_strategy> for Strategy {
    type Error = StrategyError;

    fn try_from(value: llama_cpp_sys::ggml_numa_strategy) -> Result<Self, Self::Error> {
        use Strategy::*;
        match value {
            llama_cpp_sys::GGML_NUMA_STRATEGY_DISABLED => Ok(DISABLED),
            llama_cpp_sys::GGML_NUMA_STRATEGY_DISTRIBUTE => Ok(DISTRIBUTE),
            llama_cpp_sys::GGML_NUMA_STRATEGY_ISOLATE => Ok(ISOLATE),
            llama_cpp_sys::GGML_NUMA_STRATEGY_NUMACTL => Ok(NUMACTL),
            llama_cpp_sys::GGML_NUMA_STRATEGY_MIRROR => Ok(MIRROR),
            llama_cpp_sys::GGML_NUMA_STRATEGY_COUNT => Ok(COUNT),
            strategy => Err(StrategyError::NonsupportValue { from: strategy }),
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
