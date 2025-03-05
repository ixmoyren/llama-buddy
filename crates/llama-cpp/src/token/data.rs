use super::Token;
use std::ptr;

/// `llama_token_data` 的包装
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct TokenData {
    raw: llama_cpp_sys::llama_token_data,
}

impl TokenData {
    #[must_use]
    pub fn new(Token(id): Token, logit: f32, p: f32) -> Self {
        TokenData {
            raw: llama_cpp_sys::llama_token_data { id, logit, p },
        }
    }

    #[must_use]
    pub fn id(&self) -> Token {
        Token(self.raw.id)
    }

    #[must_use]
    pub fn logit(&self) -> f32 {
        self.raw.logit
    }

    #[must_use]
    pub fn p(&self) -> f32 {
        self.raw.p
    }

    pub fn set_id(&mut self, Token(t): Token) {
        self.raw.id = t;
    }

    pub fn set_logit(&mut self, logit: f32) {
        self.raw.logit = logit;
    }

    pub fn set_p(&mut self, p: f32) {
        self.raw.p = p;
    }
}

/// `llama_token_data_array` 的包装
#[derive(Debug, Clone, PartialEq)]
pub struct TokenDataArray {
    data: Vec<TokenData>,
    selected: Option<usize>,
    sorted: bool,
}

impl TokenDataArray {
    pub fn new(data: Vec<TokenData>, sorted: bool) -> Self {
        Self {
            data,
            selected: None,
            sorted,
        }
    }

    pub fn from_iter<T>(data: T, sorted: bool) -> TokenDataArray
    where
        T: IntoIterator<Item = TokenData>,
    {
        Self::new(data.into_iter().collect(), sorted)
    }

    #[must_use]
    pub fn selected_token(&self) -> Option<Token> {
        self.data.get(self.selected?).map(TokenData::id)
    }

    pub(crate) unsafe fn modify_as_c_llama_token_data_array<T>(
        &mut self,
        modify: impl FnOnce(&mut llama_cpp_sys::llama_token_data_array) -> T,
    ) -> T {
        let size = self.data.len();
        let data = self
            .data
            .as_mut_ptr()
            .cast::<llama_cpp_sys::llama_token_data>();

        let mut c_llama_token_data_array = llama_cpp_sys::llama_token_data_array {
            data,
            size,
            selected: self.selected.and_then(|s| s.try_into().ok()).unwrap_or(-1),
            sorted: self.sorted,
        };

        let result = modify(&mut c_llama_token_data_array);

        assert!(
            c_llama_token_data_array.size <= self.data.capacity(),
            "Size of the returned array exceeds the data buffer's capacity!"
        );
        if !ptr::eq(c_llama_token_data_array.data, data) {
            ptr::copy(
                c_llama_token_data_array.data,
                data,
                c_llama_token_data_array.size,
            );
        }
        self.data.set_len(c_llama_token_data_array.size);

        self.sorted = c_llama_token_data_array.sorted;
        self.selected = c_llama_token_data_array
            .selected
            .try_into()
            .ok()
            .filter(|&s| s < self.data.len());

        result
    }
}
