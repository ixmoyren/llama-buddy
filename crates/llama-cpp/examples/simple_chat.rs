use anyhow::Context;
use clap::Parser;
use llama_cpp::{
    batch::Batch,
    context::ContextParams,
    model::{Message, ModelParams},
    runtime::Runtime,
    sampler::Sampler,
};
use rustyline::{
    Cmd, Completer, CompletionType, EditMode, Helper, Hinter, KeyEvent, Validator,
    completion::FilenameCompleter,
    error::ReadlineError,
    highlight::{CmdKind, Highlighter, MatchingBracketHighlighter},
    hint::HistoryHinter,
    validate::MatchingBracketValidator,
};
use std::{
    borrow::{
        Cow,
        Cow::{Borrowed, Owned},
    },
    io::{Write, stdout},
    path::PathBuf,
    process::exit,
};

#[derive(Parser, Debug)]
#[command(version, long_about = None)]
#[command(about = "A simple cli program for llama.cpp")]
struct Cli {
    // 本地模型所在路径
    #[arg(short = 'm', long, help = "The path of the model file")]
    model: PathBuf,
    // 文本上下文大小
    #[arg(
        short = 't',
        long,
        default_value = "2048",
        help = "The amount of text context"
    )]
    text: u32,
    #[arg(
        long = "ngl",
        default_value = "99",
        help = "The number of layers to offload to the GPU"
    )]
    layer: i32,
}

fn main() -> anyhow::Result<()> {
    let Cli {
        model: mode_path,
        text: text_context,
        layer: gpu_layer,
        ..
    } = Cli::parse();

    // 动态加载后端
    let runtime = Runtime::load_all();

    // 获取默认的模型参数
    let model_params = ModelParams::default();
    // 设置卸载到 GPU 的层数
    model_params.with_n_gpu_layers(gpu_layer);

    // 从文件中加载模型
    let model = runtime
        .load_model_from_file(mode_path, &model_params)
        .context("Failed to load model from the path")?;

    // 获取模型的词汇表
    let vocab = model.vocab();
    // 获取默认的上下文参数
    let context_params = ContextParams::default()
        .with_n_ctx(text_context)
        .with_n_batch(text_context);

    // 通过模型和上下文参数，初始化上下文
    let mut context = runtime.new_context(&model, context_params)?;

    // 设置采样器
    let min_p_sampler = Sampler::init_from_min_p(0.05_f32, 1);
    let temp_sampler = Sampler::init_from_temp(0.8_f32);
    let dist_sampler = Sampler::init_from_dist(u32::MAX);
    let mut sampler = Sampler::from_chain([min_p_sampler, temp_sampler, dist_sampler], true);

    // 设置行编辑器
    let rustyline_config = rustyline::Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Vi)
        .build();

    let rustyline_helper = RustyLineHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter::new(),
        colored_prompt: "".to_owned(),
        validator: MatchingBracketValidator::new(),
    };

    let mut rustyline = rustyline::Editor::with_config(rustyline_config)?;
    rustyline.set_helper(Some(rustyline_helper));
    rustyline.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchBackward);
    rustyline.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchForward);
    if rustyline.load_history("history.txt").is_err() {
        println!("No previous history!");
    }

    let mut count = 1;
    let mut messages = Vec::<Message>::new();
    // 获取模板
    let template = model.chat_template(None)?;
    loop {
        let prompt = format!("{count}>> ");
        rustyline.helper_mut().context("No helper!")?.colored_prompt =
            format!("\x1b[1;32m{prompt}\x1b[0m");
        let readline = rustyline.readline(&prompt);
        match readline {
            Ok(line) => {
                rustyline.add_history_entry(line.as_str())?;
                let message = Message::try_new("user", line)?;
                messages.push(message);
                let prompt = model.apply_chat_template(&template, messages.as_slice(), true)?;
                let n_ctx_used = context.kv_cache_seq_pos_max(0) + 1;
                let is_first = n_ctx_used == 0;
                let tokens = vocab.tokenize(prompt, is_first, true)?;
                let mut batch = Batch::get_one(&tokens)?;
                let mut response = String::new();
                loop {
                    let n_ctx = context.n_ctx();
                    if n_ctx_used + batch.n_tokens() > n_ctx as i32 {
                        eprintln!("context size exceeded!");
                        exit(0);
                    }
                    context.decode(&mut batch)?;
                    let new_token = sampler.sample(&context, -1);
                    if vocab.is_eog_token(new_token) {
                        break;
                    }
                    let piece = vocab.token_to_piece(&new_token, 0, true)?;
                    response += &piece;
                    print!("{piece}");
                    // print! 不会自动刷新缓冲区，要确保消息立即显示在控制台上，需要手动刷新
                    stdout().flush()?;
                    batch = Batch::get_one(&[new_token])?;
                }
                let message = Message::try_new("assistant", response)?;
                messages.push(message);
                model.apply_chat_template(&template, messages.as_slice(), false)?;
                stdout().flush()?;
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupted");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("Encountered Eof");
                break;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        }
        count += 1;
    }

    Ok(())
}

#[derive(Helper, Completer, Validator, Hinter)]
struct RustyLineHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Highlighter for RustyLineHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }
}
