use syd::{Backup, Config, Result};
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let should_restore = args.get(1).map_or(false, |arg| arg == "--restore");

    let config = Config::new(
        "~/syd/",
        "~/.config/syd/",
        "syd.conf",
    )?;
    
    let backup = Backup::new(config)?;

    if should_restore {
        backup.create_backup_folder()?;
        backup.restore_files()?;
    } else {
        backup.create_backup_folder()?;
        backup.backup_files()?;
        backup.init_git_repo()?;
        backup.commit_and_push()?;
    }
    
    Ok(())
}
