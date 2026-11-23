mod commands;
mod core;
mod utils;

use std::{num, path, process::ExitCode};

use clap::Parser as _;
use commands::{check, insert, prune, version};
use utils::replace_file;

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
    #[command(about = "Insert a phase and renumber later phases", group(clap::ArgGroup::new("phase-input").required(true).args(["title", "from"])))]
    Insert {
        position: num::NonZeroUsize,
        title: Option<String>,
        #[arg(
            long,
            value_name = "FILE",
            help = "Import one phase from a Markdown file"
        )]
        from: Option<path::PathBuf>,
        #[arg(long, help = "Print the proposed plan without writing it")]
        dry_run: bool,
    },
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
    let result = match Arguments::parse().command {
        Some(Command::Check) => {
            return match check::run(file).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("{PLAN_FILE}:{error}");
                    ExitCode::FAILURE
                }
            };
        }
        Some(Command::Insert {
            position,
            title,
            from,
            dry_run,
        }) => {
            let mode = edit_mode(dry_run);
            let title = title.as_deref();
            let from = from.as_deref();
            let result = insert::run(file, position, title, from, mode).await;
            edit_result(result)
        }
        Some(Command::Prune { dry_run }) => {
            let mode = edit_mode(dry_run);
            let result = prune::run(file, mode).await;
            edit_result(result)
        }
        Some(Command::Version) => {
            let output = version::run();
            print!("{}", output.text);
            return if output.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
        None => return ExitCode::SUCCESS,
    };
    match result {
        Ok(replace_file::Outcome::NoChange) => ExitCode::SUCCESS,
        Ok(replace_file::Outcome::Preview(updated)) => {
            print!("{updated}");
            ExitCode::SUCCESS
        }
        Ok(replace_file::Outcome::Saved(path)) => {
            println!("Backup: {}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{PLAN_FILE}:{error}");
            ExitCode::FAILURE
        }
    }
}

fn edit_mode(dry_run: bool) -> replace_file::Mode {
    if dry_run {
        replace_file::Mode::Preview
    } else {
        replace_file::Mode::Apply
    }
}

fn edit_result(
    result: Result<replace_file::Outcome, impl std::fmt::Display>,
) -> Result<replace_file::Outcome, String> {
    match result {
        Ok(outcome) => Ok(outcome),
        Err(error) => {
            let message = error.to_string();
            Err(message)
        }
    }
}
