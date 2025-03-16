use crate::token::Token;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

/// `llama_vocab` 的包装
pub struct Vocabulary {
    raw: NonNull<llama_cpp_sys::llama_vocab>,
}

impl Vocabulary {
    pub fn raw_mut(&self) -> *mut llama_cpp_sys::llama_vocab {
        self.raw.as_ptr()
    }

    #[must_use]
    pub fn token_bos(&self) -> Token {
        unsafe { llama_cpp_sys::llama_token_bos(self.raw_mut()) }.into()
    }

    #[must_use]
    pub fn token_eos(&self) -> Token {
        unsafe { llama_cpp_sys::llama_token_eos(self.raw_mut()) }.into()
    }

    #[must_use]
    pub fn token_nl(&self) -> Token {
        unsafe { llama_cpp_sys::llama_token_nl(self.raw_mut()) }.into()
    }

    #[must_use]
    pub fn is_eog_token(&self, token: Token) -> bool {
        unsafe { llama_cpp_sys::llama_token_is_eog(self.raw_mut(), token.raw()) }
    }
}

impl From<*mut llama_cpp_sys::llama_vocab> for Vocabulary {
    fn from(value: *mut llama_cpp_sys::llama_vocab) -> Self {
        Self {
            raw: NonNull::new(value).expect("Non-null pointer"),
        }
    }
}

impl From<llama_cpp_sys::llama_vocab> for Vocabulary {
    fn from(mut value: llama_cpp_sys::llama_vocab) -> Self {
        let raw = &mut value as _;
        Self {
            raw: NonNull::new(raw).expect("Non-null pointer"),
        }
    }
}

impl From<NonNull<llama_cpp_sys::llama_vocab>> for Vocabulary {
    fn from(value: NonNull<llama_cpp_sys::llama_vocab>) -> Self {
        Self { raw: value }
    }
}

impl Deref for Vocabulary {
    type Target = llama_cpp_sys::llama_vocab;

    fn deref(&self) -> &Self::Target {
        unsafe { self.raw.as_ref() }
    }
}

impl DerefMut for Vocabulary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.raw.as_mut() }
    }
}
