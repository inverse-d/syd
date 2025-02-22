use clap::{Parser, Subcommand};
use env_logger;
use syd::{Backup, Config, Result};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Backup {
        #[arg(short, long)]
        dry_run: bool,
    },
    Restore {
        #[arg(short, long)]
        force: bool,
    },
    Status,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let config = Config::new("~/syd/", "~/.config/syd/", "syd.conf")?;
    let backup = Backup::new(config)?;

    match &cli.command {
        Commands::Backup { dry_run } => {
            if *dry_run {
                println!("Dry run - would backup files");
            } else {
                backup.backup_files()?;
            }
        }
        Commands::Restore { force } => {
            if *force || prompt_confirmation("Are you sure you want to restore files?") {
                backup.restore_files()?;
            }
        }
        Commands::Status => {
            backup.status()?;
        }
    }

    Ok(())
}

fn prompt_confirmation(message: &str) -> bool {
    println!("{} [y/N]", message);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim().to_lowercase() == "y"
}
