mod cmd;
mod config;
mod db;
mod error;
mod service;
mod utils;

use crate::cmd::{
    config::output,
    init::{InitArgs, init_local_registry},
    pull::{PullArgs, pull_model_from_registry},
    simple_run::{SimpleRunArgs, simple_run_a_model},
    update::{UpdateArgs, update_local_registry},
};
use clap::{
    Parser, Subcommand,
    builder::{Styles, styling::AnsiColor},
};
use tracing::Level;

const CLI_HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Blue.on_default().bold())
    .usage(AnsiColor::Blue.on_default().bold())
    .literal(AnsiColor::White.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Parser)]
#[command(about = "llama-buddy cli interface for related operations")]
#[command(version, long_about = None, styles = CLI_HELP_STYLES)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Output the default configuration")]
    Config,
    #[command(about = "Init local registry")]
    Init(InitArgs),
    #[command(about = "Pull model from remote registry")]
    Pull(PullArgs),
    #[command(about = "Update local registry")]
    Update(UpdateArgs),
    #[command(about = "Simple run a model")]
    SimpleRun(SimpleRunArgs),
    // 列出可用的模型 list
    // 展示模型详细信息 show
    // 查找模型 search
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Config => output().await,
        Commands::Init(args) => init_local_registry(args).await,
        Commands::Pull(args) => pull_model_from_registry(args).await,
        Commands::Update(args) => update_local_registry(args).await,
        Commands::SimpleRun(args) => simple_run_a_model(args).await,
    }
}
