//！直接启动一个模型

use crate::{
    config::{Config as LLamaBuddyConfig, Data},
    db, service,
    utils::rustyline::{EditorExt, new_rustyline},
};
use clap::Args;
use llama_cpp::{
    batch::Batch,
    context::ContextParams,
    model::{Message, ModelParams},
    runtime::Runtime,
    sampler::Sampler,
};
use rustyline::error::ReadlineError;
use std::{
    fs,
    io::{Write, stdout},
    process::exit,
};
use tracing::error;

pub async fn simple_run_a_model(
    SimpleRunArgs {
        name,
        category,
        text,
        layer,
    }: SimpleRunArgs,
) {
    // 首先从配置文件中获取到本地注册表相关的信息
    let (
        LLamaBuddyConfig {
            data: Data { path: data_path },
            ..
        },
        ..,
    ) = LLamaBuddyConfig::try_config_path().expect("Couldn't get the config");
    // 构建相关数据库链接
    let sqlite_dir = data_path.join("sqlite");
    let conn = db::open_llama_buddy_db(&sqlite_dir).expect("Couldn't open sqlite file");
    // 检查一下有没有完成初始化，没有完成初始化，那么应该在完成初始化之后才能够拉取
    if !db::check_llama_buddy_init_completed(&conn).expect("Couldn't check init whatever completed")
    {
        error!("Initialization should be ensured to be completed");
        return;
    }
    // 检验模型资源是否正常拉取
    let (model_name, _category) = service::model::final_name_and_category(&conn, &name, category)
        .expect("Couldn't get model name and category");
    if !db::model::check_pull_completed(&conn, &model_name)
        .expect("Couldn't check model pull completed")
    {
        error!("Model {model_name} should be ensured to be pulled");
        return;
    }
    // 通过模型名获取到模型和模板所在的位置
    let (path, template) = db::model::get_model_params(&conn, &model_name)
        .expect("Couldn't get model path template params");
    let Some(path) = path else {
        error!("Model's path is none, should be ensured have path");
        return;
    };
    let template =
        template.map(|path| fs::read_to_string(path).expect("Couldn't to read template"));
    // 构建一个编辑器
    let mut rustyline = new_rustyline(&sqlite_dir);

    // 加载一个后端
    let runtime = Runtime::load_all();
    // 获取一个默认的参数
    let model_params = ModelParams::default().with_n_gpu_layers(layer);
    // 从文件中模型
    let model = runtime
        .load_model_from_file(path, &model_params)
        .expect("Couldn't load model");
    let context_params = ContextParams::default().with_n_ctx(text).with_n_batch(text);
    // 初始化上下文
    let mut context = runtime
        .new_context(&model, context_params)
        .expect("Failed to create a model context");
    // 设置采样器
    let min_p_sampler = Sampler::init_from_min_p(0.05_f32, 1);
    let temp_sampler = Sampler::init_from_temp(0.8_f32);
    let dist_sampler = Sampler::init_from_dist(u32::MAX);
    let mut sampler = Sampler::from_chain([min_p_sampler, temp_sampler, dist_sampler], true);
    let template = &model
        .chat_template(None)
        .expect("Failed to get a chat template from model");
    // 获取模型的词汇表
    let vocab = model.vocab();
    let mut messages = Vec::<Message>::new();
    loop {
        rustyline.colored_prompt("\x1b[1;32mQ>> \x1b[0m");
        let readline = rustyline.readline("Q>> ");
        match readline {
            Ok(line) => {
                rustyline
                    .add_history_entry(line.as_str())
                    .expect("Failed to add history entry to line editor");
                let message = Message::try_new("user", line).expect("Failed to create new message");
                messages.push(message);
                let prompt = model
                    .apply_chat_template(&template, messages.as_slice(), true)
                    .expect("Failed to apply chat template to model");
                let n_ctx_used = context.kv_cache_seq_pos_max(0) + 1;
                let is_first = n_ctx_used == 0;
                let tokens = vocab
                    .tokenize(prompt, is_first, true)
                    .expect("Failed to get tokens from vocab");
                let mut batch =
                    Batch::get_one(&tokens).expect("Failed to create a new batch by tokens");
                let mut response = String::new();
                loop {
                    let n_ctx = context.n_ctx();
                    if n_ctx_used + batch.n_tokens() > n_ctx as i32 {
                        eprintln!("context size exceeded!");
                        exit(0);
                    }
                    context.decode(&mut batch).expect("Failed to decode token");
                    let new_token = sampler.sample(&context, -1);
                    if vocab.is_eog_token(new_token) {
                        break;
                    }
                    let piece = vocab
                        .token_to_piece(&new_token, 0, true)
                        .expect("Failed to get new piece from token");
                    response += &piece;
                    print!("{piece}");
                    // print! 不会自动刷新缓冲区，要确保消息立即显示在控制台上，需要手动刷新
                    stdout().flush().expect("Failed to flush to stdout");
                    batch = Batch::get_one(&[new_token])
                        .expect("Failed to create a new batch by new token");
                }
                let message =
                    Message::try_new("assistant", response).expect("Failed to create new message");
                messages.push(message);
                model
                    .apply_chat_template(&template, messages.as_slice(), false)
                    .expect("Failed to apply chat template");
                stdout().flush().expect("Failed to flush to stdout");
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
    }
}

#[derive(Args)]
pub struct SimpleRunArgs {
    #[arg(short = 'n', long = "name", help = "The name of mode")]
    pub name: String,
    #[arg(
        short = 'c',
        long = "category",
        help = "The category of mode, If the version of the mode is not provided, the default value is obtained from the local registry"
    )]
    pub category: Option<String>,
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
