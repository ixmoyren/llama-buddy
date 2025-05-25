use crate::{
    context::{Context, ContextParams},
    error::{
        EmbeddingsError, LlamaAdapterLoraRemoveError, LlamaAdapterLoraSetError,
        LlamaContextLoadError, LlamaModelLoadError,
    },
    ggml_numa::Strategy,
    model::{AdapterLora, Model, ModelParams},
    sampler::Sampler,
    token::{Token, TokenData, TokenDataVec},
};
use std::{
    ffi::{CString, c_char, c_void},
    path::{Path, PathBuf},
    ptr::NonNull,
    slice,
    sync::OnceLock,
};
use tracing::info;

static BACKEND_INITIALIZED: OnceLock<BackendInitializedType> = OnceLock::new();

/// 用于初始化 llama.cpp
#[derive(Debug, Eq, PartialEq)]
pub struct Runtime;

impl Runtime {
    #[tracing::instrument(level = "info")]
    pub fn init() -> Self {
        let _ = BACKEND_INITIALIZED.get_or_init(|| {
            unsafe {
                llama_cpp_sys::llama_backend_init();
            }
            info!("Initialized llama.cpp backend (without numa).");
            BackendInitializedType::Default
        });
        Self
    }

    #[tracing::instrument(level = "info")]
    pub fn load_all() -> Self {
        let _ = BACKEND_INITIALIZED.get_or_init(|| {
            unsafe {
                llama_cpp_sys::ggml_backend_load_all();
            }
            info!("Initialized llama.cpp backend (without numa).");
            BackendInitializedType::LoadAll
        });
        Self
    }

    #[tracing::instrument(level = "info")]
    pub fn load_all_from_path(path: PathBuf) -> Self {
        let _ = BACKEND_INITIALIZED.get_or_init(|| {
            let dir_path = path.clone().into_os_string().into_string().unwrap();
            let dir_path = CString::new(dir_path).unwrap();
            unsafe {
                llama_cpp_sys::ggml_backend_load_all_from_path(dir_path.as_ptr());
            }
            info!("Initialized llama.cpp backend (without numa).");
            BackendInitializedType::LoadAllFromPath(path)
        });
        Self
    }

    #[tracing::instrument(level = "info")]
    pub fn init_with_ggml_numa(strategy: Strategy) -> Self {
        let _ = BACKEND_INITIALIZED.get_or_init(|| {
            let strategy = strategy.into();
            unsafe {
                llama_cpp_sys::llama_numa_init(strategy);
            }
            info!("Initialize the llama numa with strategy: {strategy}.");
            BackendInitializedType::GgmlNumaStrategy
        });
        Self
    }

    /// 是否支持 GPU
    pub fn support_gpu_offload(&self) -> bool {
        unsafe { llama_cpp_sys::llama_supports_gpu_offload() }
    }

    /// 是否支持 mmap
    pub fn support_mmap(&self) -> bool {
        unsafe { llama_cpp_sys::llama_supports_mmap() }
    }

    /// 是否支持将模型锁定在内存里面
    pub fn support_mlock(&self) -> bool {
        unsafe { llama_cpp_sys::llama_supports_mlock() }
    }

    /// 不输出日志
    pub fn void_logs(&mut self) {
        unsafe extern "C" fn void_log(
            _level: llama_cpp_sys::ggml_log_level,
            _text: *const c_char,
            _user_data: *mut c_void,
        ) {
        }

        unsafe {
            llama_cpp_sys::llama_log_set(Some(void_log), std::ptr::null_mut());
        }
    }

    #[tracing::instrument(skip_all, fields(params))]
    pub fn load_model_from_file(
        &self,
        path: impl AsRef<Path>,
        params: &ModelParams,
    ) -> Result<Model, LlamaModelLoadError> {
        let path = path.as_ref();
        debug_assert!(Path::new(path).exists(), "{path:?} does not exist");
        let path = path
            .to_str()
            .ok_or(LlamaModelLoadError::PathToStr(path.to_owned()))?;

        let cstr = CString::new(path)?;
        let llama_model =
            unsafe { llama_cpp_sys::llama_load_model_from_file(cstr.as_ptr(), params.raw()) };

        let model = NonNull::new(llama_model).ok_or(LlamaModelLoadError::NullReturn)?;

        tracing::debug!(?path, "Loaded model");
        Ok(model.into())
    }

    pub fn new_context(
        &self,
        model: &Model,
        params: ContextParams,
    ) -> Result<Context, LlamaContextLoadError> {
        let context =
            unsafe { llama_cpp_sys::llama_new_context_with_model(model.raw_mut(), params.raw()) };
        let context = NonNull::new(context).ok_or(LlamaContextLoadError::NullReturn)?;

        Ok(Context::new(context, params.embeddings()))
    }

    pub fn set_adapter_lora(
        &self,
        context: &mut Context,
        adapter: &mut AdapterLora,
        scale: f32,
    ) -> Result<(), LlamaAdapterLoraSetError> {
        let err_code = unsafe {
            llama_cpp_sys::llama_set_adapter_lora(context.raw_mut(), adapter.raw_mut(), scale)
        };
        if err_code != 0 {
            return Err(LlamaAdapterLoraSetError::ErrorReturn(err_code));
        }

        tracing::debug!("Set lora adapter");
        Ok(())
    }

    pub fn remove_adapter_lora(
        &self,
        context: &mut Context,
        adapter: &mut AdapterLora,
    ) -> Result<(), LlamaAdapterLoraRemoveError> {
        let err_code =
            unsafe { llama_cpp_sys::llama_rm_adapter_lora(context.raw_mut(), adapter.raw_mut()) };
        if err_code != 0 {
            return Err(LlamaAdapterLoraRemoveError::ErrorReturn(err_code));
        }

        tracing::debug!("Remove lora adapter");
        Ok(())
    }

    pub fn embeddings_seq_ith(
        &self,
        model: &Model,
        context: &mut Context,
        i: i32,
    ) -> Result<&[f32], EmbeddingsError> {
        if !context.embeddings_enabled() {
            return Err(EmbeddingsError::NotEnabled);
        }

        let n_embd = usize::try_from(model.n_embd()).expect("n_embd does not fit into a usize");

        unsafe {
            let embedding = llama_cpp_sys::llama_get_embeddings_seq(context.raw_mut(), i);

            // Technically also possible whenever `i >= max(batch.n_seq)`, but can't check that here.
            if embedding.is_null() {
                Err(EmbeddingsError::NonePoolType)
            } else {
                Ok(slice::from_raw_parts(embedding, n_embd))
            }
        }
    }

    pub fn embeddings_ith(
        &self,
        model: &Model,
        context: &mut Context,
        i: i32,
    ) -> Result<&[f32], EmbeddingsError> {
        if !context.embeddings_enabled() {
            return Err(EmbeddingsError::NotEnabled);
        }

        let n_embd = usize::try_from(model.n_embd()).expect("n_embd does not fit into a usize");

        unsafe {
            let embedding = llama_cpp_sys::llama_get_embeddings_ith(context.raw_mut(), i);
            // Technically also possible whenever `i >= batch.n_tokens`, but no good way of checking `n_tokens` here.
            if embedding.is_null() {
                Err(EmbeddingsError::LogitsNotEnabled)
            } else {
                Ok(slice::from_raw_parts(embedding, n_embd))
            }
        }
    }

    pub fn candidates(
        &self,
        model: &Model,
        context: &Context,
    ) -> impl Iterator<Item = TokenData> + '_ {
        (0_i32..)
            .zip(self.logits(model, context))
            .map(|(i, logit)| {
                let token = Token::new(i);
                TokenData::new(token, *logit, 0_f32)
            })
    }

    #[must_use]
    pub fn token_data_vec(&self, model: &Model, context: &Context) -> TokenDataVec {
        TokenDataVec::from_iter(self.candidates(model, context), false)
    }

    #[must_use]
    pub fn logits(&self, model: &Model, context: &Context) -> &[f32] {
        let data = unsafe { llama_cpp_sys::llama_get_logits(context.raw_mut()) };
        assert!(!data.is_null(), "logits data for last token is null");
        let vocab = model.vocab();
        let len =
            usize::try_from(vocab.token_quantity()).expect("n_vocab does not fit into a usize");

        unsafe { slice::from_raw_parts(data, len) }
    }

    pub fn candidates_ith(
        &self,
        model: &Model,
        context: &Context,
        i: i32,
    ) -> impl Iterator<Item = TokenData> + '_ {
        (0_i32..)
            .zip(self.logits_ith(model, context, i))
            .map(|(i, logit)| {
                let token = Token::new(i);
                TokenData::new(token, *logit, 0_f32)
            })
    }

    #[must_use]
    pub fn token_data_vec_ith(&self, model: &Model, context: &Context, i: i32) -> TokenDataVec {
        TokenDataVec::from_iter(self.candidates_ith(model, context, i), false)
    }

    #[must_use]
    pub fn logits_ith(&self, model: &Model, context: &Context, i: i32) -> &[f32] {
        assert!(
            context.initialized_logits().contains(&i),
            "logit {i} is not initialized. only {:?} is",
            context.initialized_logits()
        );
        assert!(
            context.n_ctx() > u32::try_from(i).expect("i does not fit into a u32"),
            "n_ctx ({}) must be greater than i ({})",
            context.n_ctx(),
            i
        );

        let data = unsafe { llama_cpp_sys::llama_get_logits_ith(context.raw_mut(), i) };
        let vocab = model.vocab();
        let len =
            usize::try_from(vocab.token_quantity()).expect("n_vocab does not fit into a usize");

        unsafe { slice::from_raw_parts(data, len) }
    }

    #[must_use]
    pub fn sampler_from_grammar(model: &Model, grammar_str: &str, grammar_root: &str) -> Sampler {
        let grammar_str = CString::new(grammar_str).unwrap();
        let grammar_root = CString::new(grammar_root).unwrap();
        let vocab = model.vocab();

        unsafe {
            llama_cpp_sys::llama_sampler_init_grammar(
                vocab.raw_mut(),
                grammar_str.as_ptr(),
                grammar_root.as_ptr(),
            )
        }
        .into()
    }

    #[must_use]
    pub fn sampler_from_grammar_lazy(
        model: &Model,
        grammar_str: &str,
        grammar_root: &str,
        trigger_words: impl IntoIterator<Item = impl AsRef<[u8]>>,
        trigger_tokens: &[Token],
    ) -> Sampler {
        let grammar_str = CString::new(grammar_str).unwrap();
        let grammar_root = CString::new(grammar_root).unwrap();

        let trigger_word_cstrings: Vec<CString> = trigger_words
            .into_iter()
            .map(|word| CString::new(word.as_ref()).unwrap())
            .collect();

        let mut trigger_word_ptrs: Vec<*const c_char> =
            trigger_word_cstrings.iter().map(|cs| cs.as_ptr()).collect();

        let vocab = model.vocab();

        unsafe {
            llama_cpp_sys::llama_sampler_init_grammar_lazy(
                vocab.raw_mut(),
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
    pub fn sampler_from_dry(
        model: &Model,
        multiplier: f32,
        base: f32,
        allowed_length: i32,
        penalty_last_n: i32,
        seq_breakers: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Sampler {
        let seq_breakers: Vec<CString> = seq_breakers
            .into_iter()
            .map(|s| CString::new(s.as_ref()).expect("A sequence breaker contains null bytes"))
            .collect();
        let mut seq_breaker_pointers: Vec<*const c_char> =
            seq_breakers.iter().map(|s| s.as_ptr()).collect();

        let vocab = model.vocab();

        unsafe {
            llama_cpp_sys::llama_sampler_init_dry(
                vocab.raw_mut(),
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
}

impl Drop for Runtime {
    #[tracing::instrument(level = "info")]
    fn drop(&mut self) {
        if let Some(_init_type) = BACKEND_INITIALIZED.get() {
            unsafe {
                llama_cpp_sys::llama_backend_free();
                info!("Freed llama.cpp backend");
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum BackendInitializedType {
    Default,
    LoadAll,
    LoadAllFromPath(PathBuf),
    GgmlNumaStrategy,
}
