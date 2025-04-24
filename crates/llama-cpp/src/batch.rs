use crate::{error::BatchAddError, token::Token};

/// A safe wrapper around `llama_batch`.
#[derive(Debug)]
pub struct Batch {
    raw: llama_cpp_sys::llama_batch,
    allocated: usize,
    initialized_logits: Vec<i32>,
}

impl Batch {
    pub fn logits(&self) -> &Vec<i32> {
        self.initialized_logits.as_ref()
    }

    pub fn raw(&self) -> llama_cpp_sys::llama_batch {
        self.raw
    }

    pub fn clear(&mut self) {
        self.raw.n_tokens = 0;
        self.initialized_logits.clear();
    }

    pub fn add(
        &mut self,
        token: Token,
        pos: llama_cpp_sys::llama_pos,
        seq_ids: &[i32],
        logits: bool,
    ) -> Result<(), BatchAddError> {
        if self.allocated < usize::try_from(self.n_tokens() + 1)? {
            return Err(BatchAddError::InsufficientSpace(self.allocated));
        }
        let offset = self.raw.n_tokens;
        let offset_usize = usize::try_from(offset)?;
        unsafe {
            self.raw.token.add(offset_usize).write(token.raw());
            self.raw.pos.add(offset_usize).write(pos);
            self.raw
                .n_seq_id
                .add(offset_usize)
                .write(llama_cpp_sys::llama_seq_id::try_from(seq_ids.len())?);
            for (i, seq_id) in seq_ids.iter().enumerate() {
                let tmp = *self.raw.seq_id.add(offset_usize);
                tmp.add(i).write(*seq_id);
            }
            self.raw.logits.add(offset_usize).write(i8::from(logits));
        }
        if logits {
            self.initialized_logits.push(offset);
        } else {
            self.initialized_logits.retain(|l| l != &offset);
        }
        self.raw.n_tokens += 1;

        Ok(())
    }

    pub fn add_sequence(
        &mut self,
        tokens: &[Token],
        seq_id: i32,
        logits_all: bool,
    ) -> Result<(), BatchAddError> {
        let n_tokens_0 = usize::try_from(self.raw.n_tokens)?;
        let n_tokens = tokens.len();

        if self.allocated < n_tokens_0 + n_tokens {
            return Err(BatchAddError::InsufficientSpace(self.allocated));
        }

        let last_index = llama_cpp_sys::llama_pos::try_from(n_tokens.saturating_sub(1))?;
        for (i, token) in (0..).zip(tokens.iter()) {
            self.add(*token, i, &[seq_id], logits_all || i == last_index)?;
        }

        Ok(())
    }

    #[must_use]
    pub fn new(n_tokens: usize, n_seq_max: i32) -> Self {
        let n_tokens_i32 = i32::try_from(n_tokens).expect("cannot fit n_tokens into an i32");
        let batch = unsafe { llama_cpp_sys::llama_batch_init(n_tokens_i32, 0, n_seq_max) };

        Batch {
            allocated: n_tokens,
            initialized_logits: vec![],
            raw: batch,
        }
    }

    pub fn get_one(tokens: &[Token]) -> Result<Self, BatchAddError> {
        if tokens.is_empty() {
            return Err(BatchAddError::EmptyBuffer);
        }
        let len = i32::try_from(tokens.len())?;
        let batch = unsafe {
            let ptr = tokens.as_ptr() as *mut i32;
            llama_cpp_sys::llama_batch_get_one(ptr, len)
        };
        let batch = Self {
            allocated: 0,
            initialized_logits: vec![len - 1],
            raw: batch,
        };
        Ok(batch)
    }

    #[must_use]
    pub fn n_tokens(&self) -> i32 {
        self.raw.n_tokens
    }
}

impl Drop for Batch {
    fn drop(&mut self) {
        unsafe {
            if self.allocated > 0 {
                llama_cpp_sys::llama_batch_free(self.raw);
            }
        }
    }
}
