mod chat;
mod lora;
mod params;

pub use chat::*;
pub use lora::*;
pub use params::*;

use crate::{
    error::{
        ApplyChatTemplateError, ChatTemplateError, LlamaAdapterLoraInitError,
        StringConversionError, TokenConversionError,
    },
    token::{Token, TokenAttr, TokenAttrs},
    vocabulary::Vocabulary,
};
use std::{
    ffi::{CStr, CString},
    num::NonZeroU16,
    ops::{Deref, DerefMut},
    os::raw::{c_char, c_int},
    path::Path,
    ptr,
    ptr::NonNull,
};

/// `llama_model` 的包装
#[derive(Debug, Clone)]
// 保证单一字段的结构体的内存布局和其唯一字段的完全一致，无填充数据
#[repr(transparent)]
pub struct Model {
    // 需要保证字段指向的指针不是空指针，明确必须指向有效的内存位置
    raw: NonNull<llama_cpp_sys::llama_model>,
}

unsafe impl Send for Model {}

unsafe impl Sync for Model {}

impl Model {
    pub fn raw_mut(&self) -> *mut llama_cpp_sys::llama_model {
        self.raw.as_ptr()
    }

    pub fn vocab(&self) -> Vocabulary {
        let raw = unsafe { llama_cpp_sys::llama_model_get_vocab(self.raw.as_ptr()) };
        raw.into()
    }

    #[must_use]
    pub fn n_ctx_train(&self) -> u32 {
        let n_ctx_train = unsafe { llama_cpp_sys::llama_n_ctx_train(self.raw.as_ptr()) };
        u32::try_from(n_ctx_train).expect("n_ctx_train fits into an u32")
    }

    pub fn tokens(
        &self,
        special: Special,
    ) -> impl Iterator<Item = (Token, Result<String, TokenConversionError>)> + '_ {
        let vocab = self.vocab();
        let quantity = vocab.token_quantity();
        (0..quantity)
            .map(Token::new)
            .map(move |llama_token| (llama_token, self.token_to_str(llama_token, special)))
    }

    #[must_use]
    pub fn decode_start_token(&self) -> Token {
        unsafe { llama_cpp_sys::llama_model_decoder_start_token(self.raw.as_ptr()) }.into()
    }

    pub fn token_to_str(
        &self,
        token: Token,
        special: Special,
    ) -> Result<String, TokenConversionError> {
        let bytes = self.token_to_bytes(token, special)?;
        Ok(String::from_utf8(bytes)?)
    }

    pub fn token_to_bytes(
        &self,
        token: Token,
        special: Special,
    ) -> Result<Vec<u8>, TokenConversionError> {
        match self.token_to_bytes_with_size(token, 8, special, None) {
            Err(TokenConversionError::InsufficientBufferSpace(i)) => {
                self.token_to_bytes_with_size(token, (-i).try_into()?, special, None)
            }
            x => x,
        }
    }

    pub fn tokens_to_str(
        &self,
        tokens: &[Token],
        special: Special,
    ) -> Result<String, TokenConversionError> {
        let mut builder: Vec<u8> = Vec::with_capacity(tokens.len() * 4);
        for piece in tokens
            .iter()
            .copied()
            .map(|t| self.token_to_bytes(t, special))
        {
            builder.extend_from_slice(&piece?);
        }
        Ok(String::from_utf8(builder)?)
    }

    pub fn str_to_token(
        &self,
        str: &str,
        add_bos: AddBos,
    ) -> Result<Vec<Token>, StringConversionError> {
        let add_bos = match add_bos {
            AddBos::Always => true,
            AddBos::Never => false,
        };

        let tokens_estimation = std::cmp::max(8, (str.len() / 2) + usize::from(add_bos));
        let mut buffer: Vec<Token> = Vec::with_capacity(tokens_estimation);

        let c_string = CString::new(str)?;
        let buffer_capacity = c_int::try_from(buffer.capacity()).map_err(|_| {
            StringConversionError::BufferCapacity(
                "Buffer capacity should fit into a c_int".to_owned(),
            )
        })?;

        let vocab = self.vocab();

        let size = unsafe {
            llama_cpp_sys::llama_tokenize(
                vocab.raw_mut(),
                c_string.as_ptr(),
                c_int::try_from(c_string.as_bytes().len())?,
                buffer.as_mut_ptr().cast::<llama_cpp_sys::llama_token>(),
                buffer_capacity,
                add_bos,
                true,
            )
        };

        let size = if size.is_negative() {
            buffer.reserve_exact(usize::try_from(-size).map_err(|_| {
                StringConversionError::BufferCapacity("value is too larger".to_owned())
            })?);
            unsafe {
                llama_cpp_sys::llama_tokenize(
                    self.vocab().raw_mut(),
                    c_string.as_ptr(),
                    c_int::try_from(c_string.as_bytes().len())?,
                    buffer.as_mut_ptr().cast::<llama_cpp_sys::llama_token>(),
                    -size,
                    add_bos,
                    true,
                )
            }
        } else {
            size
        };

        let size = usize::try_from(size).map_err(|_| {
            StringConversionError::BufferCapacity("Size is positive and usize".to_owned())
        })?;
        unsafe { buffer.set_len(size) }
        Ok(buffer)
    }

    #[must_use]
    pub fn token_attr(&self, token: Token) -> TokenAttrs {
        let vocab = self.vocab();
        let token_type =
            unsafe { llama_cpp_sys::llama_token_get_attr(vocab.raw_mut(), token.raw()) };
        TokenAttrs::try_from(token_type).expect("token type is valid")
    }

    pub fn token_to_str_with_size(
        &self,
        token: Token,
        buffer_size: usize,
        special: Special,
    ) -> Result<String, TokenConversionError> {
        let bytes = self.token_to_bytes_with_size(token, buffer_size, special, None)?;
        Ok(String::from_utf8(bytes)?)
    }

    pub fn token_to_bytes_with_size(
        &self,
        token: Token,
        buffer_size: usize,
        special: Special,
        lstrip: Option<NonZeroU16>,
    ) -> Result<Vec<u8>, TokenConversionError> {
        let vocab = self.vocab();
        if token == vocab.token_nl() {
            return Ok(b"\n".to_vec());
        }

        let attrs = self.token_attr(token);
        if attrs.is_empty()
            || attrs.intersects(TokenAttr::Unknown | TokenAttr::Byte | TokenAttr::Unused)
            || attrs.contains(TokenAttr::Control)
                && (token == vocab.token_bos() || token == vocab.token_eos())
        {
            return Ok(Vec::new());
        }

        let special = match special {
            Special::Tokenize => true,
            Special::Plaintext => false,
        };

        let string = CString::new(vec![b'*'; buffer_size]).expect("no null");
        let len = string.as_bytes().len();
        let len = c_int::try_from(len).expect("length fits into c_int");
        let buf = string.into_raw();
        let lstrip = lstrip.map_or(0, |it| i32::from(it.get()));
        let size = unsafe {
            llama_cpp_sys::llama_token_to_piece(
                vocab.raw_mut(),
                token.raw(),
                buf,
                len,
                lstrip,
                special,
            )
        };

        match size {
            0 => Err(TokenConversionError::UnknownType),
            i if i.is_negative() => Err(TokenConversionError::InsufficientBufferSpace(i)),
            size => {
                let string = unsafe { CString::from_raw(buf) };
                let mut bytes = string.into_bytes();
                let len = usize::try_from(size).expect("size is positive and fits into usize");
                bytes.truncate(len);
                Ok(bytes)
            }
        }
    }

    #[must_use]
    pub fn n_embd(&self) -> c_int {
        unsafe { llama_cpp_sys::llama_n_embd(self.raw.as_ptr()) }
    }

    pub fn size(&self) -> u64 {
        unsafe { llama_cpp_sys::llama_model_size(self.raw.as_ptr()) }
    }

    pub fn n_params(&self) -> u64 {
        unsafe { llama_cpp_sys::llama_model_n_params(self.raw.as_ptr()) }
    }

    pub fn is_recurrent(&self) -> bool {
        unsafe { llama_cpp_sys::llama_model_is_recurrent(self.raw.as_ptr()) }
    }

    pub fn n_layer(&self) -> u32 {
        u32::try_from(unsafe { llama_cpp_sys::llama_model_n_layer(self.raw.as_ptr()) }).unwrap()
    }

    pub fn n_head(&self) -> u32 {
        u32::try_from(unsafe { llama_cpp_sys::llama_model_n_head(self.raw.as_ptr()) }).unwrap()
    }

    /// Returns the rope type of the model.
    pub fn rope_type(&self) -> Option<RopeType> {
        match unsafe { llama_cpp_sys::llama_model_rope_type(self.raw.as_ptr()) } {
            llama_cpp_sys::LLAMA_ROPE_TYPE_NONE => None,
            llama_cpp_sys::LLAMA_ROPE_TYPE_NORM => Some(RopeType::Norm),
            llama_cpp_sys::LLAMA_ROPE_TYPE_NEOX => Some(RopeType::NeoX),
            llama_cpp_sys::LLAMA_ROPE_TYPE_MROPE => Some(RopeType::MRope),
            llama_cpp_sys::LLAMA_ROPE_TYPE_VISION => Some(RopeType::Vision),
            rope_type => {
                tracing::error!(rope_type = rope_type, "Unexpected rope type from llama.cpp");
                None
            }
        }
    }

    fn chat_template_impl(&self, capacity: usize) -> Result<Template, ChatTemplateError> {
        let mut chat_temp = vec![b'*'; capacity];
        let chat_name = c"tokenizer.chat_template";

        let ret = unsafe {
            llama_cpp_sys::llama_model_meta_val_str(
                self.raw.as_ptr(),
                chat_name.as_ptr(),
                chat_temp.as_mut_ptr() as *mut c_char,
                chat_temp.len(),
            )
        };

        if ret < 0 {
            return Err(ChatTemplateError::MissingTemplate(ret));
        }

        let returned_len = ret as usize;

        if ret as usize >= capacity {
            return Err(ChatTemplateError::RetryWithLargerBuffer(returned_len));
        }

        assert_eq!(
            chat_temp.get(returned_len),
            Some(&0),
            "should end with null byte"
        );

        chat_temp.resize(returned_len + 1, 0);

        let template = unsafe { CString::from_vec_with_nul_unchecked(chat_temp) };

        Ok(template.into())
    }

    pub fn chat_template_by_meta(&self) -> Result<Template, ChatTemplateError> {
        match self.chat_template_impl(200) {
            Ok(t) => Ok(t),
            Err(ChatTemplateError::RetryWithLargerBuffer(actual_len)) => {
                match self.chat_template_impl(actual_len + 1) {
                    Ok(t) => Ok(t),
                    Err(ChatTemplateError::RetryWithLargerBuffer(unexpected_len)) => panic!(
                        "Was told that the template length was {actual_len} but now it's {unexpected_len}"
                    ),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn chat_template(&self, name: Option<String>) -> Result<Template, ChatTemplateError> {
        let key = if let Some(name) = name {
            let c_str = CString::new(name)?;
            c_str.as_ptr()
        } else {
            ptr::null() as *const c_char
        };

        let c_str_ptr = unsafe { llama_cpp_sys::llama_model_chat_template(self.raw_mut(), key) };

        if c_str_ptr.is_null() {
            return Err(ChatTemplateError::NulTemplate);
        }

        let c_str = unsafe { CStr::from_ptr(c_str_ptr) };

        let template = CString::from(c_str);

        Ok(template.into())
    }

    pub fn lora_adapter_init(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<AdapterLora, LlamaAdapterLoraInitError> {
        let path = path.as_ref();
        debug_assert!(Path::new(path).exists(), "{path:?} does not exist");

        let path = path
            .to_str()
            .ok_or(LlamaAdapterLoraInitError::PathToStr(path.to_path_buf()))?;

        let cstr = CString::new(path)?;
        let adapter =
            unsafe { llama_cpp_sys::llama_adapter_lora_init(self.raw.as_ptr(), cstr.as_ptr()) };

        let adapter = NonNull::new(adapter).ok_or(LlamaAdapterLoraInitError::NullReturn)?;

        tracing::debug!(?path, "Initialized lora adapter");
        Ok(adapter.into())
    }

    #[tracing::instrument(skip_all)]
    pub fn apply_chat_template(
        &self,
        template: &Template,
        chats: &[Message],
        add_ass: bool,
    ) -> Result<String, ApplyChatTemplateError> {
        // 预设缓冲区长度为全部信息字符长度的两倍
        let message_length = chats.iter().fold(0, |acc, c| {
            acc + c.role.to_bytes().len() + c.content.to_bytes().len()
        });
        let mut buff = vec![0_u8; message_length * 2];

        let chats = chats
            .iter()
            .map(Into::into)
            .collect::<Vec<llama_cpp_sys::llama_chat_message>>();
        let tmpl_ptr = template.as_ptr();
        let buff_len = i32::try_from(buff.len())?;
        let res = unsafe {
            llama_cpp_sys::llama_chat_apply_template(
                tmpl_ptr,
                chats.as_ptr(),
                chats.len(),
                add_ass,
                buff.as_mut_ptr() as *mut c_char,
                buff_len,
            )
        };

        // 从 llama-cpp 中可知返回 -1 是遇到了无法处理的的模板类型
        if res == -1 {
            return Err(ApplyChatTemplateError::UnknownTemplate);
        }

        // 缓冲区过小，结果有被截断，重新申请一个缓冲区，重新调用 llama_chat_apply_template 方法
        if res > buff_len {
            let new_len =
                usize::try_from(res).map_err(|_| ApplyChatTemplateError::ResUnreasonable)?;
            buff.resize(new_len, 0_u8);
            let _res = unsafe {
                llama_cpp_sys::llama_chat_apply_template(
                    tmpl_ptr,
                    chats.as_ptr(),
                    chats.len(),
                    add_ass,
                    buff.as_mut_ptr() as *mut c_char,
                    buff.len().try_into()?,
                )
            };
        }

        let len = usize::try_from(res).map_err(|_| ApplyChatTemplateError::ResUnreasonable)?;
        buff.truncate(len);

        Ok(String::from_utf8(buff)?)
    }
}

impl From<*mut llama_cpp_sys::llama_model> for Model {
    fn from(value: *mut llama_cpp_sys::llama_model) -> Self {
        Self {
            raw: NonNull::new(value).expect("Non-null pointer"),
        }
    }
}

impl From<llama_cpp_sys::llama_model> for Model {
    fn from(mut value: llama_cpp_sys::llama_model) -> Self {
        let raw = &mut value as _;
        Self {
            raw: NonNull::new(raw).expect("Non-null pointer"),
        }
    }
}

impl From<NonNull<llama_cpp_sys::llama_model>> for Model {
    fn from(value: NonNull<llama_cpp_sys::llama_model>) -> Self {
        Self { raw: value }
    }
}

impl Deref for Model {
    type Target = llama_cpp_sys::llama_model;

    fn deref(&self) -> &Self::Target {
        unsafe { self.raw.as_ref() }
    }
}

impl DerefMut for Model {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.raw.as_mut() }
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe { llama_cpp_sys::llama_model_free(self.raw.as_ptr()) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RopeType {
    Norm,
    NeoX,
    MRope,
    Vision,
}

/// How to determine if we should prepend a bos token to tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddBos {
    /// Add the beginning of stream token to the start of the string.
    Always,
    /// Do not add the beginning of stream token to the start of the string.
    Never,
}

/// How to determine if we should tokenize special tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Special {
    /// Allow tokenizing special and/or control tokens which otherwise are not exposed and treated as plaintext. Does not insert a leading space.
    Tokenize,
    /// Treat special and/or control tokens as plaintext.
    Plaintext,
}
