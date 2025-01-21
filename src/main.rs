use syd::{Backup, Config, Result};

fn main() -> Result<()> {
    let config = Config::new(
        "~/syd/",
        "~/.config/syd/",
        "syd.conf",
    )?;
    
    let git_repo = config.git_repo.clone();
    let backup = Backup::new(config)?;
    backup.create_backup_folder()?;
    backup.backup_files()?;
    backup.init_git_repo()?;
    backup.commit_and_push(&git_repo)?;
    
    Ok(())
}
