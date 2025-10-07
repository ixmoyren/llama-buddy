use crate::token::Token;
use snafu::prelude::*;
use std::{
    ffi::{CString, c_char},
    num::TryFromIntError,
    ops::{Deref, DerefMut},
    ptr,
    ptr::{NonNull, slice_from_raw_parts},
};

const TOKEN_TEXT_MAX_LENGTH: usize = i32::MAX as usize;

/// `llama_vocab` 的包装
pub struct Vocabulary {
    raw: NonNull<llama_cpp_sys::llama_vocab>,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum VocabularyError {
    #[snafu(display("The buffer is too small: {size}, when token to piece"))]
    TokenToPieceBufTooSmall { size: i32 },
    #[snafu(display("The text length({size}) exceeds the buffer"))]
    TokenizeTextTooLong { size: usize },
    #[snafu(display("The supplied bytes contain an internal 0 byte"))]
    TokenizeCstrNul { source: std::ffi::NulError },
    #[snafu(display("Token too many: {size}, when tokenize"))]
    TokenizeTokenTooMany { size: i32 },
    #[snafu(display(
        "The number of tokens obtained after word segmentation is not equal to the pre-calculated value({value})"
    ))]
    TokenizeFailed { value: i32 },
    #[snafu(display("The buffer is too small: {size}, when detokenize"))]
    DetokenizeBufTooSmall { size: i32 },
    #[snafu(display("Unknown token type: {value}"))]
    UnknownTokenType { value: i32 },
    #[snafu(display("Can't fit {from} to usize"))]
    I32IntoUsize { from: i32, source: TryFromIntError },
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

    /// Token Id -> Piece.
    ///
    /// 使用提供的上下文中词汇表
    ///
    /// 不将空结束符写入缓冲区
    ///
    /// 用户可以在复制之前跳过 `lstrip` 前导空格（当使用 `add_space_prefix` 编码或者解码多个 token 时非常有用）
    ///
    /// special 如果为 true, 在输出中呈现特殊的 token
    pub fn token_to_piece(
        &self,
        token: &Token,
        lstrip: i32,
        special: bool,
    ) -> Result<String, VocabularyError> {
        // 提供一个初始缓冲区
        let mut buf_size = 256;
        let mut buf = vec![0_u8; buf_size];

        let vocab = self.raw_mut();
        let token = token.raw();

        let mut piece_len = unsafe {
            llama_cpp_sys::llama_token_to_piece(
                vocab,
                token,
                buf.as_mut_ptr() as *mut c_char,
                buf_size as i32,
                lstrip,
                special,
            )
        };

        // 缓冲区太小，调整大小重试
        if piece_len < 0 {
            buf_size =
                usize::try_from(-piece_len).context(I32IntoUsizeSnafu { from: -piece_len })?;
            buf = vec![0u8; buf_size];
            piece_len = unsafe {
                llama_cpp_sys::llama_token_to_piece(
                    vocab,
                    token,
                    buf.as_mut_ptr() as *mut c_char,
                    buf_size as i32,
                    lstrip,
                    special,
                )
            };
            ensure!(
                piece_len >= 0,
                TokenToPieceBufTooSmallSnafu { size: piece_len }
            );
        }

        ensure!(piece_len > 0, UnknownTokenTypeSnafu { value: piece_len });

        unsafe {
            buf.set_len(piece_len as usize);
        }
        let str = String::from_utf8_lossy(&buf).to_string();
        Ok(str)
    }

    /// 将提供的文本转换成 token 序列
    ///
    /// 令牌指针指向的空间必须以容纳生成的 token，成功生成的 token 数量不会超过 n_tokens_max
    ///
    /// 如果模型允许，可以通过 `add_special` 添加 `BOS` 或者 `EOS` 类型的令牌
    ///
    /// 设置 parse_special 允许 tokenize 特殊或控制等字符，否则不会暴露和处理作为明文，不插入前导空格。
    pub fn tokenize(
        &self,
        text: impl AsRef<str>,
        add_special: bool,
        parse_special: bool,
    ) -> Result<Vec<Token>, VocabularyError> {
        let text = text.as_ref();
        // 检查文本长度
        let text_len = text.len();
        ensure!(
            text_len <= TOKEN_TEXT_MAX_LENGTH,
            TokenizeTextTooLongSnafu { size: text_len }
        );

        let vocab = self.raw_mut();

        let text = CString::new(text).context(TokenizeCstrNulSnafu)?;

        // 通过 token 为 null 和 n_tokens_max 为 0, 可以计算 text 中令牌的数量
        let token_len = unsafe {
            // 失败时返回一个负数，原本应该生成的 token 数量，这里是为了获取生成 token 的数量，所以要加上 -
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
        ensure!(token_len > 0, TokenizeTokenTooManySnafu { size: token_len });

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

            ensure!(
                token_len == tokenize_len,
                TokenizeFailedSnafu {
                    value: tokenize_len
                }
            );

            tokens.set_len(tokenize_len as usize);
        }

        Ok(tokens.into_iter().map(Token::from).collect())
    }

    /// 将提供的 token 序列转换为文本
    ///
    /// 如果模型允许，通过 `remove_special` 设置为 true 允许删除 `BOS` 和 `EOS` 令牌
    ///
    /// `unparse_special` 如果为 true，则在输出中呈现特殊 token
    pub fn detokenize(
        &self,
        tokens: &[Token],
        remove_special: bool,
        unparse_special: bool,
    ) -> Result<String, VocabularyError> {
        let vocab = self.raw_mut();
        let token_len = tokens.len();
        let tokens = slice_from_raw_parts(
            tokens.as_ptr() as *const llama_cpp_sys::llama_token,
            token_len,
        ) as *const llama_cpp_sys::llama_token;

        // 提供一个初始缓冲区
        let mut buf_size = 256;
        let mut buf = vec![0_u8; buf_size];

        let mut de_tokenize_len = unsafe {
            // 失败时返回一个负数，本应返回的字符/字节数
            llama_cpp_sys::llama_detokenize(
                vocab,
                tokens,
                token_len as i32,
                buf.as_mut_ptr() as *mut c_char,
                buf_size as i32,
                remove_special,
                unparse_special,
            )
        };

        // 缓冲区太小，调整大小重试
        if de_tokenize_len < 0 {
            buf_size = usize::try_from(-de_tokenize_len).context(I32IntoUsizeSnafu {
                from: -de_tokenize_len,
            })?;
            buf = vec![0_u8; buf_size];
            // 失败时返回一个负数 - 本应返回的字符/字节数
            de_tokenize_len = unsafe {
                llama_cpp_sys::llama_detokenize(
                    vocab,
                    tokens,
                    token_len as i32,
                    buf.as_mut_ptr() as *mut c_char,
                    buf_size as i32,
                    remove_special,
                    unparse_special,
                )
            };
            ensure!(
                de_tokenize_len >= 0,
                DetokenizeBufTooSmallSnafu {
                    size: de_tokenize_len
                }
            )
        }

        ensure!(
            de_tokenize_len > 0,
            UnknownTokenTypeSnafu {
                value: de_tokenize_len
            }
        );

        unsafe {
            buf.set_len(de_tokenize_len as usize);
        }
        let str = String::from_utf8_lossy(&buf).to_string();
        Ok(str)
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

/// `llama_vocab_type`
#[repr(u32)]
#[derive(Debug, Eq, Copy, Clone, PartialEq)]
pub enum VocabularyType {
    /// For models without vocab
    NONE = llama_cpp_sys::LLAMA_VOCAB_TYPE_NONE as _,
    /// LLaMA tokenizer based on byte-level BPE with byte fallback
    SPM = llama_cpp_sys::LLAMA_VOCAB_TYPE_SPM as _,
    /// GPT-2 tokenizer based on byte-level BPE
    BPE = llama_cpp_sys::LLAMA_VOCAB_TYPE_BPE as _,
    /// BERT tokenizer based on WordPiece
    WPM = llama_cpp_sys::LLAMA_VOCAB_TYPE_WPM as _,
    /// T5 tokenizer based on Unigram
    UGM = llama_cpp_sys::LLAMA_VOCAB_TYPE_UGM as _,
    /// RWKV tokenizer based on greedy tokenization
    RWKV = llama_cpp_sys::LLAMA_VOCAB_TYPE_RWKV as _,
    /// PLaMo-2 tokenizer based on Aho-Corasick with dynamic programming
    PLAMO2 = llama_cpp_sys::LLAMA_VOCAB_TYPE_PLAMO2 as _,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum VocabularyTypeError {
    #[snafu(display("Nonsupport llama vocab type: {from}"))]
    NonsupportValue {
        from: llama_cpp_sys::llama_vocab_type,
    },
}

impl TryFrom<llama_cpp_sys::llama_vocab_type> for VocabularyType {
    type Error = VocabularyTypeError;

    fn try_from(value: llama_cpp_sys::llama_vocab_type) -> Result<Self, Self::Error> {
        use self::VocabularyType::*;
        match value {
            llama_cpp_sys::LLAMA_VOCAB_TYPE_NONE => Ok(NONE),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_SPM => Ok(SPM),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_BPE => Ok(BPE),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_WPM => Ok(WPM),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_UGM => Ok(UGM),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_RWKV => Ok(RWKV),
            llama_cpp_sys::LLAMA_VOCAB_TYPE_PLAMO2 => Ok(PLAMO2),
            vocab_type => Err(VocabularyTypeError::NonsupportValue { from: vocab_type }),
        }
    }
}

impl From<VocabularyType> for llama_cpp_sys::llama_vocab_type {
    fn from(value: VocabularyType) -> Self {
        value as _
    }
}
