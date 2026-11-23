mod commands;
mod core;
mod utils;

use std::{path, process::ExitCode};

use clap::Parser as _;
use commands::{check, prune, version};

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
    #[command(about = "Remove completed phases and renumber the rest")]
    Prune {
        #[arg(long, help = "Print the proposed plan without writing it")]
        dry_run: bool,
    },
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
        Some(Command::Prune { dry_run }) => {
            let mode = if dry_run {
                prune::Mode::Preview
            } else {
                prune::Mode::Apply
            };
            match prune::run(file, mode).await {
                Ok(prune::Outcome::NoChange) => ExitCode::SUCCESS,
                Ok(prune::Outcome::Preview(updated)) => {
                    print!("{updated}");
                    ExitCode::SUCCESS
                }
                Ok(prune::Outcome::Saved(path)) => {
                    println!("Backup: {}", path.display());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{PLAN_FILE}:{error}");
                    ExitCode::FAILURE
                }
            }
        }
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
