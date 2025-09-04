use super::Token;
use crate::sampler::Sampler;
use std::ptr;

/// `llama_token_data` 的包装
#[derive(Clone, Copy, Debug)]
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
#[derive(Debug, Clone)]
pub struct TokenDataVec {
    raw: Vec<TokenData>,
    selected: Option<usize>,
    sorted: bool,
}

impl TokenDataVec {
    pub fn new(data: Vec<TokenData>, sorted: bool) -> Self {
        Self {
            raw: data,
            selected: None,
            sorted,
        }
    }

    pub fn from_iter<T>(data: T, sorted: bool) -> TokenDataVec
    where
        T: IntoIterator<Item = TokenData>,
    {
        Self::new(data.into_iter().collect(), sorted)
    }

    #[must_use]
    pub fn selected_token(&self) -> Option<Token> {
        self.raw.get(self.selected?).map(TokenData::id)
    }

    pub(crate) unsafe fn modify_by_llama_token_data_array<T>(
        &mut self,
        modify_fn: impl FnOnce(&mut llama_cpp_sys::llama_token_data_array) -> T,
    ) -> T {
        let size = self.raw.len();
        let data = self
            .raw
            .as_mut_ptr()
            .cast::<llama_cpp_sys::llama_token_data>();

        let mut data_array = llama_cpp_sys::llama_token_data_array {
            data,
            size,
            selected: self.selected.and_then(|s| s.try_into().ok()).unwrap_or(-1),
            sorted: self.sorted,
        };

        let result = modify_fn(&mut data_array);

        assert!(
            data_array.size <= self.raw.capacity(),
            "Size of the returned array exceeds the data buffer's capacity!"
        );
        if !ptr::eq(data_array.data, data) {
            unsafe {
                ptr::copy(data_array.data, data, data_array.size);
                self.raw.set_len(data_array.size);
            }
        }

        self.sorted = data_array.sorted;
        self.selected = data_array
            .selected
            .try_into()
            .ok()
            .filter(|&s| s < self.raw.len());

        result
    }

    pub fn apply_sampler(&mut self, sampler: &Sampler) {
        unsafe {
            self.modify_by_llama_token_data_array(|data_array| {
                llama_cpp_sys::llama_sampler_apply(sampler.raw_mut(), data_array);
            });
        }
    }

    #[must_use]
    pub fn with_sampler(mut self, sampler: &mut Sampler) -> Self {
        self.apply_sampler(sampler);
        self
    }

    pub fn sample_token(&mut self, seed: u32) -> Token {
        let sampler = Sampler::init_from_dist(seed);
        self.apply_sampler(&sampler);
        self.selected_token()
            .expect("Dist sampler failed to select a token!")
    }

    pub fn sample_token_greedy(&mut self) -> Token {
        let sampler = Sampler::init_from_greedy();
        self.apply_sampler(&sampler);
        self.selected_token()
            .expect("Greedy sampler failed to select a token!")
    }
}
