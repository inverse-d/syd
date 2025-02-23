use std::io::{self, Error};
use std::path::PathBuf;
use std::fs;
use simple_expand_tilde::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub backup: BackupConfig,
    pub git: GitConfig,
}

#[derive(Deserialize)]
pub struct BackupConfig {
    pub folder: String,
    pub paths: Vec<String>,
}

#[derive(Deserialize)]
pub struct GitConfig {
    pub remote_url: String,
    pub branch: String,
}

impl Config {
    pub fn load() -> io::Result<Self> {
        let config_path = expand_tilde("~/.config/syd/syd.conf")
            .ok_or_else(|| Error::new(io::ErrorKind::NotFound, "Failed to expand config path"))?;
        
        let contents = fs::read_to_string(config_path)?;
        toml::from_str(&contents).map_err(|e| Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn create_backup_folder(&self) -> io::Result<PathBuf> {
        let expanded_path = expand_tilde(&self.backup.folder)
            .ok_or_else(|| Error::new(io::ErrorKind::NotFound, "Failed to expand backup folder path"))?;
        
        if !expanded_path.exists() {
            fs::create_dir_all(&expanded_path)?;
            operations::create_local_repo(&expanded_path)
                .map_err(|e| Error::new(io::ErrorKind::Other, e.message()))?;
        }
        Ok(expanded_path)
    }
}

pub mod operations {
    use git2::{Repository, RemoteCallbacks, PushOptions};
    use std::path::PathBuf;
    use super::*;  // Add this to access Config
    use std::fs;
    use std::io::{self};  // Fix: wrap self in curly braces

    fn files_are_different(path1: &PathBuf, path2: &PathBuf) -> io::Result<bool> {
        if !path2.exists() {
            return Ok(true);
        }

        let metadata1 = fs::metadata(path1)?;
        let metadata2 = fs::metadata(path2)?;

        // Compare file sizes
        if metadata1.len() != metadata2.len() {
            return Ok(true);
        }

        // Compare modification times
        match (metadata1.modified(), metadata2.modified()) {
            (Ok(time1), Ok(time2)) => Ok(time1 != time2),
            // If we can't compare modification times, assume files are different
            _ => Ok(true)
        }
    }

    pub fn backup_dotfiles(config: &Config) -> io::Result<bool> {
        println!("Checking files for backup:");
        let backup_path = expand_tilde(&config.backup.folder)
            .ok_or_else(|| Error::new(io::ErrorKind::NotFound, "Failed to expand backup folder path"))?;

        let mut has_changes = false;
        let mut modified_count = 0;

        for path in &config.backup.paths {
            if let Some(original_path) = expand_tilde(path) {
                if original_path.exists() {
                    let file_name = original_path.file_name()
                        .ok_or_else(|| Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?;
                    
                    let backup_file = backup_path.join(file_name);
                    
                    if files_are_different(&original_path, &backup_file)? {
                        fs::copy(&original_path, &backup_file)?;
                        println!("✓ Backed up {} (updated)", path);
                        has_changes = true;
                        modified_count += 1;
                    }
                } else {
                    println!("✗ {} (not found)", path);
                }
            }
        }

        if modified_count == 0 {
            println!("No files needed backup");
        }
        
        Ok(has_changes)
    }

    pub fn create_local_repo(path: &PathBuf) -> Result<(), git2::Error> {
        if !path.join(".git").exists() {
            Repository::init(path)?;
        }
        Ok(())
    }

    pub fn push_to_git(path: &PathBuf, remote_url: &str) -> Result<(), git2::Error> {
        let repo = Repository::open(path)?;
        
        // Set up authentication for all git operations
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        // Configure remote
        if let Ok(mut remote) = repo.find_remote("origin") {
            remote.disconnect()?;
            repo.remote_delete("origin")?;
        }
        let mut remote = repo.remote("origin", remote_url)?;
        
        // Create initial branch if it doesn't exist
        if repo.find_branch("main", git2::BranchType::Local).is_err() {
            // Create and write initial commit
            let mut index = repo.index()?;
            index.add_all(["."].iter(), git2::IndexAddOption::DEFAULT, None)?;
            index.write()?;

            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            let signature = repo.signature()?;

            // Create initial commit
            repo.commit(
                Some("refs/heads/main"),  // Create main branch
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],  // No parents for initial commit
            )?;
        }

        // Stage and commit changes
        let mut index = repo.index()?;
        index.add_all(["."].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = repo.signature()?;
        let parent = repo.head()?.peel_to_commit()?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Update dotfiles",
            &tree,
            &[&parent],
        )?;

        // Push to remote
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);
        remote.push(&["refs/heads/main:refs/heads/main"], Some(&mut push_options))?;
        
        Ok(())
    }

    pub fn restore_dotfiles(config: &Config) -> io::Result<()> {
        println!("Checking files for restoration:");
        let backup_path = expand_tilde(&config.backup.folder)
            .ok_or_else(|| Error::new(io::ErrorKind::NotFound, "Failed to expand backup folder path"))?;

        let mut files_restored = false;

        for path in &config.backup.paths {
            if let Some(original_path) = expand_tilde(path) {
                let file_name = original_path.file_name()
                    .ok_or_else(|| Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?;
                
                let backup_file = backup_path.join(file_name);
                
                if backup_file.exists() {
                    if !original_path.exists() || files_are_different(&backup_file, &original_path)? {
                        if let Some(parent) = original_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        
                        fs::copy(&backup_file, &original_path)?;
                        println!("✓ Restored {} (updated)", path);
                        files_restored = true;
                    }
                } else {
                    println!("✗ Backup not found for {}", path);
                }
            }
        }

        if !files_restored {
            println!("No files needed restoration");
        }
        
        Ok(())
    }
} 