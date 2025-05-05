mod pull;

use clap::{
    builder::{styling::AnsiColor, Styles}, Parser,
    Subcommand,
};

use crate::pull::{pull_model_from_registry, PullArgs};
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
    /// Pull a model from a registry
    Pull(PullArgs),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Pull(args) => pull_model_from_registry(args).await,
    }
}
