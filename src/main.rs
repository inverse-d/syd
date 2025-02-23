use clap::Command;
use syd::{Config, operations};

fn main() {
    let matches = Command::new("syd")
        .about("Backup and restore dotfiles")
        .subcommand_required(true)
        .subcommand(Command::new("backup")
            .about("Backup dotfiles to repository"))
        .subcommand(Command::new("restore")
            .about("Restore dotfiles from repository"))
        .get_matches();

    match matches.subcommand() {
        Some(("backup", _)) => {
            let config = Config::load().expect("Failed to load config");
            let backup_path = config.create_backup_folder().unwrap();
            let has_changes = operations::backup_dotfiles(&config)
                .expect("Failed to backup dotfiles");
            if has_changes {
                operations::push_to_git(&backup_path, &config.git.remote_url)
                    .expect("Failed to push to git repository");
                println!("Changes pushed to remote repository");
            }
        }
        Some(("restore", _)) => {
            let config = Config::load().expect("Failed to load config");
            operations::restore_dotfiles(&config)
                .expect("Failed to restore dotfiles");
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }
}
