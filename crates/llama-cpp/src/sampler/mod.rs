use crate::model::Model;
use crate::token::{LogitBias, Token, TokenDataVec};
use std::borrow::Borrow;
use std::ffi::{CString, c_char};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

pub struct Sampler {
    raw: NonNull<llama_cpp_sys::llama_sampler>,
}

impl Sampler {
    pub fn raw_mut(&self) -> *mut llama_cpp_sys::llama_sampler {
        self.raw.as_ptr()
    }

    pub fn reset(&mut self) {
        unsafe {
            llama_cpp_sys::llama_sampler_reset(self.raw_mut());
        }
    }

    #[must_use]
    pub fn seed(&self) -> u32 {
        unsafe { llama_cpp_sys::llama_sampler_get_seed(self.raw_mut()) }
    }

    pub fn accept(&mut self, token: Token) {
        unsafe { llama_cpp_sys::llama_sampler_accept(self.raw_mut(), token.raw()) }
    }

    pub fn accept_tokens(&mut self, tokens: impl IntoIterator<Item = impl Borrow<Token>>) {
        for token in tokens {
            unsafe { llama_cpp_sys::llama_sampler_accept(self.raw_mut(), token.borrow().raw()) }
        }
    }

    pub fn apply_to(&self, token_data_vec: &mut TokenDataVec) {
        unsafe {
            token_data_vec.modify_by_llama_token_data_array(|data_array| {
                llama_cpp_sys::llama_sampler_apply(self.raw_mut(), data_array);
            })
        }
    }

    #[must_use]
    pub fn with_tokens(mut self, tokens: impl IntoIterator<Item = impl Borrow<Token>>) -> Self {
        self.accept_tokens(tokens);
        self
    }

    #[must_use]
    pub fn from_chain(samplers: impl IntoIterator<Item = Self>, no_perf: bool) -> Self {
        unsafe {
            let chain = llama_cpp_sys::llama_sampler_chain_init(
                llama_cpp_sys::llama_sampler_chain_params { no_perf },
            );

            for sampler in samplers {
                llama_cpp_sys::llama_sampler_chain_add(chain, sampler.raw.as_ptr());
                std::mem::forget(sampler);
            }

            chain.into()
        }
    }

    #[must_use]
    pub fn from_chain_simple(samplers: impl IntoIterator<Item = Self>) -> Self {
        Self::from_chain(samplers, false)
    }

    #[must_use]
    pub fn from_temp(t: f32) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_temp(t) }.into()
    }

    #[must_use]
    pub fn from_temp_ext(t: f32, delta: f32, exponent: f32) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_temp_ext(t, delta, exponent) }.into()
    }

    #[must_use]
    pub fn from_top_k(k: i32) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_top_k(k) }.into()
    }

    #[must_use]
    pub fn from_top_n_sigma(n: f32) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_top_n_sigma(n) }.into()
    }

    #[must_use]
    pub fn from_typical(p: f32, min_keep: usize) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_typical(p, min_keep) }.into()
    }

    #[must_use]
    pub fn from_top_p(p: f32, min_keep: usize) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_top_p(p, min_keep) }.into()
    }

    #[must_use]
    pub fn from_min_p(p: f32, min_keep: usize) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_min_p(p, min_keep) }.into()
    }

    #[must_use]
    pub fn from_xtc(p: f32, t: f32, min_keep: usize, seed: u32) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_xtc(p, t, min_keep, seed) }.into()
    }

    #[must_use]
    pub fn from_grammar(model: &Model, grammar_str: &str, grammar_root: &str) -> Self {
        let grammar_str = CString::new(grammar_str).unwrap();
        let grammar_root = CString::new(grammar_root).unwrap();

        unsafe {
            llama_cpp_sys::llama_sampler_init_grammar(
                model.vocab_ptr(),
                grammar_str.as_ptr(),
                grammar_root.as_ptr(),
            )
        }
        .into()
    }

    #[must_use]
    pub fn from_grammar_lazy(
        model: &Model,
        grammar_str: &str,
        grammar_root: &str,
        trigger_words: impl IntoIterator<Item = impl AsRef<[u8]>>,
        trigger_tokens: &[Token],
    ) -> Self {
        let grammar_str = CString::new(grammar_str).unwrap();
        let grammar_root = CString::new(grammar_root).unwrap();

        let trigger_word_cstrings: Vec<CString> = trigger_words
            .into_iter()
            .map(|word| CString::new(word.as_ref()).unwrap())
            .collect();

        let mut trigger_word_ptrs: Vec<*const c_char> =
            trigger_word_cstrings.iter().map(|cs| cs.as_ptr()).collect();

        unsafe {
            llama_cpp_sys::llama_sampler_init_grammar_lazy(
                model.vocab_ptr(),
                grammar_str.as_ptr(),
                grammar_root.as_ptr(),
                trigger_word_ptrs.as_mut_ptr(),
                trigger_word_ptrs.len(),
                trigger_tokens.as_ptr().cast(),
                trigger_tokens.len(),
            )
        }
        .into()
    }

    #[must_use]
    pub fn from_dry(
        model: &Model,
        multiplier: f32,
        base: f32,
        allowed_length: i32,
        penalty_last_n: i32,
        seq_breakers: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Self {
        let seq_breakers: Vec<CString> = seq_breakers
            .into_iter()
            .map(|s| CString::new(s.as_ref()).expect("A sequence breaker contains null bytes"))
            .collect();
        let mut seq_breaker_pointers: Vec<*const c_char> =
            seq_breakers.iter().map(|s| s.as_ptr()).collect();

        unsafe {
            llama_cpp_sys::llama_sampler_init_dry(
                model.vocab_ptr(),
                model
                    .n_ctx_train()
                    .try_into()
                    .expect("n_ctx_train exceeds i32::MAX"),
                multiplier,
                base,
                allowed_length,
                penalty_last_n,
                seq_breaker_pointers.as_mut_ptr(),
                seq_breaker_pointers.len(),
            )
        }
        .into()
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn from_penalties(
        penalty_last_n: i32,
        penalty_repeat: f32,
        penalty_freq: f32,
        penalty_present: f32,
    ) -> Self {
        unsafe {
            llama_cpp_sys::llama_sampler_init_penalties(
                penalty_last_n,
                penalty_repeat,
                penalty_freq,
                penalty_present,
            )
        }
        .into()
    }

    #[must_use]
    pub fn from_mirostat(n_vocab: i32, seed: u32, tau: f32, eta: f32, m: i32) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_mirostat(n_vocab, seed, tau, eta, m) }.into()
    }

    #[must_use]
    pub fn from_mirostat_v2(seed: u32, tau: f32, eta: f32) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_mirostat_v2(seed, tau, eta) }.into()
    }

    /// Selects a token at random based on each token's probabilities
    #[must_use]
    pub fn from_dist(seed: u32) -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_dist(seed) }.into()
    }

    #[must_use]
    pub fn from_greedy() -> Self {
        unsafe { llama_cpp_sys::llama_sampler_init_greedy() }.into()
    }

    #[must_use]
    pub fn from_logit_bias(n_vocab: i32, biases: &[LogitBias]) -> Self {
        let data = biases.as_ptr().cast::<llama_cpp_sys::llama_logit_bias>();

        unsafe { llama_cpp_sys::llama_sampler_init_logit_bias(n_vocab, biases.len() as i32, data) }
            .into()
    }
}

impl From<*mut llama_cpp_sys::llama_sampler> for Sampler {
    fn from(value: *mut llama_cpp_sys::llama_sampler) -> Self {
        Self {
            raw: NonNull::new(value).expect("Non-null pointer"),
        }
    }
}

impl From<llama_cpp_sys::llama_sampler> for Sampler {
    fn from(mut value: llama_cpp_sys::llama_sampler) -> Self {
        let value = &mut value as _;
        Self {
            raw: NonNull::new(value).expect("Non-null pointer"),
        }
    }
}

impl From<NonNull<llama_cpp_sys::llama_sampler>> for Sampler {
    fn from(value: NonNull<llama_cpp_sys::llama_sampler>) -> Self {
        Self { raw: value }
    }
}

impl Deref for Sampler {
    type Target = llama_cpp_sys::llama_sampler;

    fn deref(&self) -> &Self::Target {
        unsafe { self.raw.as_ref() }
    }
}

impl DerefMut for Sampler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.raw.as_mut() }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            llama_cpp_sys::llama_sampler_free(self.raw.as_ptr());
        }
    }
}
