use crate::error::{
    DeTokenizeError, TokenToPieceError, TokenizeError, VocabularyTypeConversionError,
};
use crate::token::Token;
use std::ffi::{CStr, CString};
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;
use std::ptr;
use std::ptr::{slice_from_raw_parts, NonNull};

/// `llama_vocab` 的包装
pub struct Vocabulary {
    raw: NonNull<llama_cpp_sys::llama_vocab>,
}

impl Vocabulary {
    pub fn raw_mut(&self) -> *mut llama_cpp_sys::llama_vocab {
        self.raw.as_ptr()
    }

    #[must_use]
    pub fn vocab_type(&self) -> VocabularyType {
        let vocab_type = unsafe { llama_cpp_sys::llama_vocab_type(self.raw_mut()) };
        VocabularyType::try_from(vocab_type).expect("invalid vocab type")
    }

    #[must_use]
    pub fn token_bos(&self) -> Token {
        unsafe { llama_cpp_sys::llama_token_bos(self.raw_mut()) }.into()
    }

    /// 模型被训练的令牌数量
    #[must_use]
    pub fn token_quantity(&self) -> i32 {
        unsafe { llama_cpp_sys::llama_vocab_n_tokens(self.raw_mut()) }
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

    pub fn token_to_piece(
        &self,
        token: Token,
        lstrip: i32,
        special: bool,
    ) -> Result<String, TokenToPieceError> {
        // 提供一个初始缓冲区
        let mut buf_size = 256;
        let mut buf = Vec::<c_char>::with_capacity(buf_size);

        let vocab = self.raw_mut();
        let token = token.raw();

        let mut piece_len = unsafe {
            llama_cpp_sys::llama_token_to_piece(
                vocab,
                token,
                buf.as_mut_ptr(),
                buf_size as i32,
                lstrip,
                special,
            )
        };

        // 缓冲区太小，调整大小重试
        if piece_len < 0 {
            buf_size = usize::try_from(-piece_len)?;
            buf = Vec::<c_char>::with_capacity(buf_size);
            piece_len = unsafe {
                llama_cpp_sys::llama_token_to_piece(
                    vocab,
                    token,
                    buf.as_mut_ptr(),
                    buf_size as i32,
                    lstrip,
                    special,
                )
            };
            if piece_len < 0 {
                return Err(TokenToPieceError::BufferTooSmall(piece_len));
            }
            unsafe {
                buf.set_len(piece_len as usize);
            }
        }
        let c_str = unsafe { CStr::from_ptr(buf.as_mut_ptr()) };
        Ok(c_str.to_str()?.to_owned())
    }

    pub fn tokenize(
        &self,
        text: impl AsRef<str>,
        add_special: bool,
        parse_special: bool,
    ) -> Result<Vec<Token>, TokenizeError> {
        let text = text.as_ref();
        // 检查文本长度
        let text_len = text.len();
        if text_len > i32::MAX as usize {
            return Err(TokenizeError::TextTooLong(text_len));
        }

        let vocab = self.raw_mut();

        let text = CString::new(text)?;

        // 通过 token 为 null 和 n_tokens_max 为 0, 可以计算 text 中令牌的数量
        let token_len = unsafe {
            -llama_cpp_sys::llama_tokenize(
                vocab,
                text.as_ptr(),
                text_len as i32,
                ptr::null_mut(),
                0,
                add_special,
                parse_special,
            )
        };

        if token_len <= 0 {
            return Err(TokenizeError::TokenTooMany(token_len));
        }

        let mut tokens = Vec::<llama_cpp_sys::llama_token>::with_capacity(token_len as usize);

        unsafe {
            let tokenize_len = llama_cpp_sys::llama_tokenize(
                vocab,
                text.as_ptr(),
                text_len as i32,
                tokens.as_mut_ptr(),
                token_len,
                add_special,
                parse_special,
            );

            if token_len != tokenize_len {
                return Err(TokenizeError::TokenizeFailed(
                    "The number of tokens obtained after word segmentation is not equal to the pre-calculated value".to_owned(),
                ));
            }
            tokens.set_len(tokenize_len as usize);
        }

        Ok(tokens.into_iter().map(Token::from).collect())
    }

    pub fn detokenize(
        &self,
        tokens: &[Token],
        remove_special: bool,
        unparse_special: bool,
    ) -> Result<String, DeTokenizeError> {
        let vocab = self.raw_mut();
        let token_len = tokens.len();
        let tokens = slice_from_raw_parts(
            tokens.as_ptr() as *const llama_cpp_sys::llama_token,
            token_len,
        ) as *const llama_cpp_sys::llama_token;

        // 提供一个初始缓冲区
        let mut buf_size = 256;
        let mut buf = Vec::<c_char>::with_capacity(buf_size);

        let mut de_tokenize_len = unsafe {
            llama_cpp_sys::llama_detokenize(
                vocab,
                tokens,
                token_len as i32,
                buf.as_mut_ptr(),
                buf_size as i32,
                remove_special,
                unparse_special,
            )
        };

        // 缓冲区太小，调整大小重试
        if de_tokenize_len < 0 {
            buf_size = usize::try_from(-de_tokenize_len)?;
            buf = Vec::<c_char>::with_capacity(buf_size);
            de_tokenize_len = unsafe {
                llama_cpp_sys::llama_detokenize(
                    vocab,
                    tokens,
                    token_len as i32,
                    buf.as_mut_ptr(),
                    buf_size as i32,
                    remove_special,
                    unparse_special,
                )
            };
            if de_tokenize_len < 0 {
                return Err(DeTokenizeError::BufferTooSmall(de_tokenize_len));
            }
            unsafe {
                buf.set_len(de_tokenize_len as usize);
            }
        }
        let c_str = unsafe { CStr::from_ptr(buf.as_mut_ptr()) };
        Ok(c_str.to_str()?.to_owned())
    }
}

impl From<*const llama_cpp_sys::llama_vocab> for Vocabulary {
    fn from(value: *const llama_cpp_sys::llama_vocab) -> Self {
        Self {
            raw: NonNull::new(value as _).expect("Non-null pointer"),
        }
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

/// a rusty equivalent of `llama_vocab_type`
#[repr(u32)]
#[derive(Debug, Eq, Copy, Clone, PartialEq)]
pub enum VocabularyType {
    NONE = llama_cpp_sys::LLAMA_VOCAB_TYPE_NONE as _,
    SPM = llama_cpp_sys::LLAMA_VOCAB_TYPE_SPM as _,
    BPE = llama_cpp_sys::LLAMA_VOCAB_TYPE_BPE as _,
    WPM = llama_cpp_sys::LLAMA_VOCAB_TYPE_WPM as _,
    UGM = llama_cpp_sys::LLAMA_VOCAB_TYPE_UGM as _,
    RWKV = llama_cpp_sys::LLAMA_VOCAB_TYPE_RWKV as _,
}

impl TryFrom<llama_cpp_sys::llama_vocab_type> for VocabularyType {
    type Error = VocabularyTypeConversionError;

    fn try_from(value: llama_cpp_sys::llama_vocab_type) -> Result<Self, Self::Error> {
        use self::VocabularyType::*;
        match value {
            llama_cpp_sys::LLAMA_VOCAB_TYPE_NONE => Ok(NONE),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_SPM => Ok(SPM),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_BPE => Ok(BPE),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_WPM => Ok(WPM),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_UGM => Ok(UGM),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_RWKV => Ok(RWKV),
            unknown => Err(VocabularyTypeConversionError::UnknownValue(unknown)),
        }
    }
}
