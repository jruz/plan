mod commands;
mod core;
mod utils;

use std::{path, process::ExitCode};

use clap::Parser as _;
use commands::{check, version};

const PLAN_FILE: &str = "PLAN.md";

#[derive(Debug, clap::Parser)]
#[command(version = version::text(), about = "Work with PLAN.md files")]
struct Arguments {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    #[command(about = "Validate PLAN.md without changing it")]
    Check,
    #[command(about = "Show version and build metadata")]
    Version,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let file = path::Path::new(PLAN_FILE);
    match Arguments::parse().command {
        Some(Command::Check) => match check::run(file).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{PLAN_FILE}:{error}");
                ExitCode::FAILURE
            }
        },
        Some(Command::Version) => {
            let output = version::run();
            print!("{}", output.text);
            if output.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        None => ExitCode::SUCCESS,
    }
}
