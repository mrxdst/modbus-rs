mod address;
pub mod args;
pub mod connect;
pub mod server;
mod util;

use std::process::ExitCode;

use args::Cli;
use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.host.as_str() {
        "server" => server::run(cli).await,
        _ => connect::run(cli).await,
    };

    if let Err(err) = result {
        eprintln!("Error: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
