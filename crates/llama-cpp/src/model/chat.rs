use snafu::prelude::*;
use std::{
    ffi::{CStr, CString},
    num::TryFromIntError,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Template(CString);
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum TemplateError {
    #[snafu(display("The supplied bytes contain an internal 0 byte when new a template"))]
    TemplateContainZeroByte { source: std::ffi::NulError },
    #[snafu(display("The original CString cannot be converted to str"))]
    TemplateCannotCovertToUtf8 { source: std::str::Utf8Error },
    #[snafu(display("The original bytes cannot be converted to string"))]
    TemplateCannotCovertToString { source: std::string::FromUtf8Error },
    #[snafu(display("The model has no template"))]
    NulTemplate,
    #[snafu(display("Unsupported template types"))]
    UnknownTemplate,
    #[snafu(display("The model has no meta val returned code({code})"))]
    MissingTemplate { code: i32 },
    #[snafu(display("The template buffer({size}) is too large, capacity is {capacity}"))]
    TemplateRetryWithLargerBuffer { size: usize, capacity: usize },
    #[snafu(display("The value({from}) is positive and fits into usize"))]
    TemplateI32IntoUsize { from: i32, source: TryFromIntError },
    #[snafu(display("The value({from}) is positive and fits into i32"))]
    TemplateUsizeIntoI32 {
        from: usize,
        source: TryFromIntError,
    },
}

impl Template {
    pub fn new(template: impl AsRef<str>) -> Result<Self, TemplateError> {
        Ok(Self(
            CString::new(template.as_ref()).context(TemplateContainZeroByteSnafu)?,
        ))
    }

    pub fn as_c_str(&self) -> &CStr {
        &self.0
    }

    pub fn to_str(&self) -> Result<&str, TemplateError> {
        Ok(self.0.to_str().context(TemplateCannotCovertToUtf8Snafu)?)
    }

    pub fn to_owned(&self) -> Result<String, TemplateError> {
        self.to_str().map(str::to_owned)
    }

    pub fn to_string(&self) -> Result<String, TemplateError> {
        self.to_str().map(str::to_string)
    }
}

impl From<CString> for Template {
    fn from(value: CString) -> Self {
        Self(value)
    }
}

impl Deref for Template {
    type Target = CString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Template {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// `llama_chat_message` 包装
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Message {
    pub role: CString,
    pub content: CString,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum MessageError {
    #[snafu(display(
        "The supplied bytes(from {role}: \"{from}\") contain an internal 0 byte when new a message"
    ))]
    MessageContainZeroByte {
        from: String,
        role: String,
        source: std::ffi::NulError,
    },
}

impl Message {
    pub fn try_new(role: impl AsRef<str>, content: impl AsRef<str>) -> Result<Self, MessageError> {
        let role = role.as_ref();
        let content = content.as_ref();
        Ok(Self {
            role: CString::new(role).context(MessageContainZeroByteSnafu {
                from: role.to_owned(),
                role: "role".to_owned(),
            })?,
            content: CString::new(content).context(MessageContainZeroByteSnafu {
                from: content.to_owned(),
                role: "content".to_owned(),
            })?,
        })
    }
}

impl From<Message> for llama_cpp_sys::llama_chat_message {
    fn from(Message { role, content }: Message) -> Self {
        Self {
            role: role.as_ptr(),
            content: content.as_ptr(),
        }
    }
}

impl From<&Message> for llama_cpp_sys::llama_chat_message {
    fn from(Message { role, content }: &Message) -> Self {
        Self {
            role: role.as_ptr(),
            content: content.as_ptr(),
        }
    }
}
