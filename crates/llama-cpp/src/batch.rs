use crate::{Result, token::Token};
use snafu::prelude::*;
use std::{num::TryFromIntError, slice};

/// 一个 `llama_batch` 的 wrapper
///
/// llama_batch 是 llama_encode/llama_decode 的输入数据
///
/// 一个 llama_batch 对象包含一个或者多个序列，它提供的 `token 数组`/`embd 数组`/`pos 数组`等数组的长度一定要和 n_tokens 一样长
///
/// - token  : 输入的 token id（当 embd 为 NULL 时使用）
/// - embd   : 内嵌的 token（即大小为 n_embd 的浮点向量）（当标记为 NULL 时使用）
/// - pos    : token 在序列中的位置
///            (如果设置为 NULL，则标记位置将由 `llama_encode`/`llama_decode` 自动跟踪)
/// - seq_id : token 所属的序列
///            (如果设置为 NULL，则假定序列 ID 为 0)
/// - logits : 如果为零，则不会输出相应 token
///            ( 如果设置为NULL:
///               - 如果是 embd：输出所有 token
///               - 如果不是 embd：只输出最后一个 token
///            )
///
#[derive(Debug)]
pub struct Batch {
    raw: llama_cpp_sys::llama_batch,
    allocated_space: i32,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum BatchError {
    #[snafu(display("Empty token slice"))]
    EmptyTokenSlice,
    #[snafu(display("Allocated space insufficient, quantity is {size}"))]
    AllocatedSpaceInsufficient { size: i32 },
    #[snafu(display("Can't fit {from} to i32"))]
    UsizeIntoI32 {
        from: usize,
        source: TryFromIntError,
    },
}

impl Batch {
    #[must_use]
    pub fn allocated_space(&self) -> i32 {
        self.allocated_space
    }

    #[must_use]
    pub fn n_tokens(&self) -> i32 {
        self.raw.n_tokens
    }

    pub fn logits(&self) -> &[i8] {
        unsafe { slice::from_raw_parts_mut(self.raw.logits, self.allocated_space() as usize) }
    }

    pub fn raw(&self) -> llama_cpp_sys::llama_batch {
        self.raw
    }

    pub fn clear(&mut self) {
        self.raw.n_tokens = 0;
    }

    pub fn add(&mut self, token: Token, pos: i32, seq_ids: &[i32], logits: bool) -> Result<()> {
        let n_tokens = self.n_tokens() + 1;
        ensure!(
            self.allocated_space() >= n_tokens,
            AllocatedSpaceInsufficientSnafu {
                size: self.allocated_space()
            }
        );
        let offset = self.n_tokens() as usize;
        let len = seq_ids.len();
        let seq_len = i32::try_from(len).context(UsizeIntoI32Snafu { from: len })?;
        unsafe {
            self.raw.token.add(offset).write(token.raw());
            self.raw.pos.add(offset).write(pos);
            self.raw.n_seq_id.add(offset).write(seq_len);
            for (i, seq_id) in seq_ids.iter().enumerate() {
                let seq_ids = *self.raw.seq_id.add(offset);
                seq_ids.add(i).write(*seq_id);
            }
            self.raw.logits.add(offset).write(i8::from(logits));
        }
        self.raw.n_tokens += 1;

        Ok(())
    }

    pub fn add_sequence(&mut self, tokens: &[Token], seq_id: i32, logits_all: bool) -> Result<()> {
        let raw_n_tokens = self.n_tokens() as usize;
        let tokens_len = tokens.len();

        ensure!(
            self.allocated_space() as usize >= raw_n_tokens + tokens_len,
            AllocatedSpaceInsufficientSnafu {
                size: self.allocated_space()
            }
        );

        let len = tokens_len.saturating_sub(1);
        let last_index =
            llama_cpp_sys::llama_pos::try_from(len).context(UsizeIntoI32Snafu { from: len })?;
        for (i, token) in (0..).zip(tokens.iter()) {
            self.add(*token, i, &[seq_id], logits_all || i == last_index)?;
        }

        Ok(())
    }

    /// 在堆上分配一批 token，token 的数量由 `allocated_space` 指定
    ///
    /// 每个令牌最多可以分配 `max_seq_size` 序列id
    ///
    /// 必须使用 `llama_batch_free` 释放
    ///
    /// 所有成员都未初始化
    #[must_use]
    pub fn new(allocated_space: i32, max_seq_size: i32) -> Self {
        assert!(
            allocated_space >= 0,
            "The allocated_space must be greater than zero"
        );
        assert!(
            max_seq_size >= 0,
            "The max_seq_size must be greater than zero"
        );
        let batch = unsafe { llama_cpp_sys::llama_batch_init(allocated_space, 0, max_seq_size) };

        Batch {
            raw: batch,
            allocated_space,
        }
    }

    pub fn get_one(tokens: &[Token]) -> Result<Self> {
        ensure!(tokens.len() > 0, EmptyTokenSliceSnafu);
        let len = tokens.len() as i32;
        let batch = unsafe { llama_cpp_sys::llama_batch_get_one(tokens.as_ptr() as _, len) };
        let batch = Self {
            raw: batch,
            allocated_space: 0,
        };
        Ok(batch)
    }
}

impl Drop for Batch {
    fn drop(&mut self) {
        unsafe {
            if self.allocated_space > 0 {
                llama_cpp_sys::llama_batch_free(self.raw);
            }
        }
    }
}
