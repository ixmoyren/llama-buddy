use super::Token;

/// `llama_logit_bias` 包装器
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct LogitBias {
    raw: llama_cpp_sys::llama_logit_bias,
}

impl LogitBias {
    #[must_use]
    pub fn new(Token(t): Token, bias: f32) -> Self {
        Self {
            raw: llama_cpp_sys::llama_logit_bias { token: t, bias },
        }
    }

    #[must_use]
    pub fn token(&self) -> Token {
        Token(self.raw.token)
    }

    #[must_use]
    pub fn bias(&self) -> f32 {
        self.raw.bias
    }

    pub fn set_token(&mut self, Token(t): Token) {
        self.raw.token = t;
    }

    pub fn set_bias(&mut self, bias: f32) {
        self.raw.bias = bias;
    }
}
