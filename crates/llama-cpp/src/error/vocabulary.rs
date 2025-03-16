use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum VocabularyTypeConversionError {
    #[error("Unknown Value {0}")]
    UnknownValue(llama_cpp_sys::llama_vocab_type),
}
