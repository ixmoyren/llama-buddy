use anyhow::Context;
use clap::Parser;
use llama_cpp::{
    batch::Batch, context::ContextParams, model::ModelParams, runtime::Runtime, sampler::Sampler,
    utils::ggml_time_us,
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, long_about = None)]
#[command(about = "A simple cli program for llama.cpp")]
struct Cli {
    // 本地模型所在路径
    #[arg(short = 'm', long, help = "The path of the model file")]
    model: PathBuf,
    // 要预测的令牌数量
    #[arg(
        short = 'p',
        long,
        default_value = "32",
        help = "The number of tokens to predict"
    )]
    quantity: i32,
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
        quantity: token_quantity,
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
    // 提供提示词
    let prompt = "Hello my name is";
    // 对提示词进行分词
    let tokens = vocab.tokenize(prompt, true, true)?;

    // 设置上下文大小
    let n_ctx = tokens.len() as u32 + token_quantity as u32 - 1;
    // 获取默认的上下文参数
    let context_params = ContextParams::default()
        .with_n_ctx(n_ctx)
        .with_n_batch(token_quantity as u32)
        .with_n_perf(false);

    // 通过模型和上下文参数，初始化上下文
    let mut context = runtime.new_context(&model, context_params)?;

    // 使用贪心采样器
    let greedy_sampler = Sampler::init_from_greedy();
    // 开启性能计数器，并且将采样器添加到采样链中
    let mut sampler = Sampler::from_chain([greedy_sampler], false);

    // 逐词打印提示词
    for token in &tokens {
        let token_str = vocab.token_to_piece(token, 0, true)?;
        // 打印令牌对应的字符串
        print!("{token_str}");
    }

    // 记录主循环开始的时间
    let start = ggml_time_us();
    // 为提示词准备一个批次
    let mut batch = Batch::get_one(&tokens)?;
    // 需要使用一个可变的上下文
    let context = &mut context;
    // 需要一个可变的采样器
    let sampler = &mut sampler;
    let mut decode_num = 0;
    let mut pos = 0_i32;
    while pos + batch.n_tokens() < tokens.len() as i32 + token_quantity {
        // 使用变换器模型评估当前批次
        context.decode(&mut batch)?;
        // 更新当前位置
        pos += batch.n_tokens();
        // 采样下一个令牌
        let token = sampler.sample(context, -1);
        // 将令牌转换成字符串
        let token_str = &vocab.token_to_piece(&token, 0, true)?;
        println!("{token_str}");
        // 为下一个批次准备一个新的令牌
        batch = Batch::get_one(&[token])?;
        decode_num += 1;
    }
    let end = ggml_time_us();
    println!(
        "decoded {decode_num} tokens in {} s, speed: {} t/s",
        (end - start) as f64 / 100_0000.0,
        decode_num as f64 / ((end - start) as f64 / 100_000.0),
    );

    Ok(())
}
