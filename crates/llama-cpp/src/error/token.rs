use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum TokenTypeConversionError {
    #[error("Unknown Value {0}")]
    UnknownValue(std::ffi::c_uint),
}
