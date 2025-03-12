use clap::Command;
use syd::{Config, operations};
use env_logger;

fn main() {
    env_logger::init();

    let matches = Command::new("syd")
        .about("Backup and restore dotfiles")
        .subcommand_required(true)
        .subcommand(Command::new("backup")
            .about("Backup dotfiles to repository"))
        .subcommand(Command::new("restore")
            .about("Restore dotfiles from repository"))
        .subcommand(Command::new("list")
            .about("List tracked dotfiles and their status"))
        .get_matches();

    if let Err(e) = run(matches) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(matches: clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        Some(("backup", _)) => {
            let config = Config::load()?;
            let backup_path = config.create_backup_folder()?;
            let has_changes = operations::backup_dotfiles(&config)?;
            if has_changes {
                operations::push_to_git(&backup_path, &config.git.remote_url)?;
                println!("Changes pushed to remote repository");
            }
        }
        Some(("restore", _)) => {
            let config = Config::load()?;
            operations::restore_dotfiles(&config)?;
        }
        Some(("list", _)) => {
            let config = Config::load()?;
            operations::list_dotfiles(&config)?;
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }
    Ok(())
}
