use std::fmt::{Debug, Display, Formatter};

/// `llama_perf_context_data` 的包装.
#[derive(Clone, Copy, Debug)]
pub struct Perf {
    raw: llama_cpp_sys::llama_perf_context_data,
}

impl Perf {
    #[must_use]
    pub fn new(
        t_start_ms: f64,
        t_load_ms: f64,
        t_p_eval_ms: f64,
        t_eval_ms: f64,
        n_p_eval: i32,
        n_eval: i32,
        n_reused: i32,
    ) -> Self {
        Self {
            raw: llama_cpp_sys::llama_perf_context_data {
                t_start_ms,
                t_load_ms,
                t_p_eval_ms,
                t_eval_ms,
                n_p_eval,
                n_eval,
                n_reused,
            },
        }
    }

    #[must_use]
    pub fn t_start_ms(&self) -> f64 {
        self.raw.t_start_ms
    }

    #[must_use]
    pub fn t_load_ms(&self) -> f64 {
        self.raw.t_load_ms
    }

    #[must_use]
    pub fn t_p_eval_ms(&self) -> f64 {
        self.raw.t_p_eval_ms
    }

    #[must_use]
    pub fn t_eval_ms(&self) -> f64 {
        self.raw.t_eval_ms
    }

    #[must_use]
    pub fn n_p_eval(&self) -> i32 {
        self.raw.n_p_eval
    }

    #[must_use]
    pub fn n_eval(&self) -> i32 {
        self.raw.n_eval
    }

    pub fn with_t_start_ms(&mut self, t_start_ms: f64) {
        self.raw.t_start_ms = t_start_ms;
    }

    pub fn with_t_load_ms(&mut self, t_load_ms: f64) {
        self.raw.t_load_ms = t_load_ms;
    }

    pub fn with_t_p_eval_ms(&mut self, t_p_eval_ms: f64) {
        self.raw.t_p_eval_ms = t_p_eval_ms;
    }

    pub fn with_t_eval_ms(&mut self, t_eval_ms: f64) {
        self.raw.t_eval_ms = t_eval_ms;
    }

    pub fn with_n_p_eval(&mut self, n_p_eval: i32) {
        self.raw.n_p_eval = n_p_eval;
    }

    pub fn with_n_eval(&mut self, n_eval: i32) {
        self.raw.n_eval = n_eval;
    }

    pub fn with_n_reused(&mut self, n_reused: i32) {
        self.raw.n_reused = n_reused;
    }
}

impl Display for Perf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "load time = {:.2} ms", self.t_load_ms())?;
        writeln!(
            f,
            "prompt eval time = {:.2} ms / {} tokens ({:.2} ms per token, {:.2} tokens per second)",
            self.t_p_eval_ms(),
            self.n_p_eval(),
            self.t_p_eval_ms() / f64::from(self.n_p_eval()),
            1e3 / self.t_p_eval_ms() * f64::from(self.n_p_eval())
        )?;
        writeln!(
            f,
            "eval time = {:.2} ms / {} runs ({:.2} ms per token, {:.2} tokens per second)",
            self.t_eval_ms(),
            self.n_eval(),
            self.t_eval_ms() / f64::from(self.n_eval()),
            1e3 / self.t_eval_ms() * f64::from(self.n_eval())
        )?;
        Ok(())
    }
}

impl From<llama_cpp_sys::llama_perf_context_data> for Perf {
    fn from(raw: llama_cpp_sys::llama_perf_context_data) -> Self {
        Self { raw }
    }
}
