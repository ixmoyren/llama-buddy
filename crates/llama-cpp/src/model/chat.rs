use crate::error::{ChatMessageError, ChatTemplateError};
use std::{
    ffi::{CStr, CString},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Template(CString);

impl Template {
    pub fn new(template: impl AsRef<str>) -> Result<Self, ChatTemplateError> {
        Ok(Self(CString::new(template.as_ref())?))
    }

    pub fn as_c_str(&self) -> &CStr {
        &self.0
    }

    pub fn to_str(&self) -> Result<&str, ChatTemplateError> {
        Ok(self.0.to_str()?)
    }

    pub fn to_owned(&self) -> Result<String, ChatTemplateError> {
        self.to_str().map(str::to_owned)
    }

    pub fn to_string(&self) -> Result<String, ChatTemplateError> {
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

impl Message {
    pub fn new(role: impl AsRef<str>, content: impl AsRef<str>) -> Result<Self, ChatMessageError> {
        Ok(Self {
            role: CString::new(role.as_ref())?,
            content: CString::new(content.as_ref())?,
        })
    }
}
