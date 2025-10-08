mod data;
mod logit_bias;

pub use data::{TokenData, TokenDataVec};
use enumflags2::{BitFlags, FromBitsError, bitflags};
pub use logit_bias::LogitBias;
use snafu::prelude::*;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

/// `llama_token` 包装
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Token(llama_cpp_sys::llama_token);

#[derive(Debug, Snafu)]
pub enum TokenError {
    #[snafu(display("Unknown value {value}"))]
    UnknownValue {
        value: std::ffi::c_uint,
        source: FromBitsError<TokenAttr>,
    },
}

impl Token {
    #[must_use]
    pub fn new(id: i32) -> Self {
        Self(id)
    }

    pub fn raw(&self) -> llama_cpp_sys::llama_token {
        self.0
    }
}

impl From<llama_cpp_sys::llama_token> for Token {
    fn from(token: llama_cpp_sys::llama_token) -> Self {
        Self(token)
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// `llama_token_type` 包装.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
#[bitflags]
#[repr(u32)]
pub enum TokenAttr {
    Unknown = llama_cpp_sys::LLAMA_TOKEN_ATTR_UNKNOWN as _,
    Unused = llama_cpp_sys::LLAMA_TOKEN_ATTR_UNUSED as _,
    Normal = llama_cpp_sys::LLAMA_TOKEN_ATTR_NORMAL as _,
    Control = llama_cpp_sys::LLAMA_TOKEN_ATTR_CONTROL as _,
    UserDefined = llama_cpp_sys::LLAMA_TOKEN_ATTR_USER_DEFINED as _,
    Byte = llama_cpp_sys::LLAMA_TOKEN_ATTR_BYTE as _,
    Normalized = llama_cpp_sys::LLAMA_TOKEN_ATTR_NORMALIZED as _,
    LStrip = llama_cpp_sys::LLAMA_TOKEN_ATTR_LSTRIP as _,
    RStrip = llama_cpp_sys::LLAMA_TOKEN_ATTR_RSTRIP as _,
    SingleWord = llama_cpp_sys::LLAMA_TOKEN_ATTR_SINGLE_WORD as _,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenAttrs(pub BitFlags<TokenAttr>);

impl Deref for TokenAttrs {
    type Target = BitFlags<TokenAttr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TokenAttrs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<llama_cpp_sys::llama_token_type> for TokenAttrs {
    type Error = TokenError;

    fn try_from(value: llama_cpp_sys::llama_token_type) -> Result<Self, Self::Error> {
        Ok(Self(
            BitFlags::from_bits(value as _).context(UnknownValueSnafu { value })?,
        ))
    }
}
