use std::path::PathBuf;
use thiserror::Error;
use serde::Deserialize;
use std::fs;

#[derive(Error, Debug)]
pub enum SydError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to expand tilde: {0}")]
    TildeExpansion(String),
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("Failed to parse config: {0}")]
    ConfigParse(String),
}

pub type Result<T> = std::result::Result<T, SydError>;

pub struct Config {
    backup_folder: PathBuf,
    config_path: PathBuf,
    pub git_repo: String,
}

#[derive(Deserialize)]
struct ConfigFile {
    git: GitConfig,
    files: FilesConfig,
}

#[derive(Deserialize)]
struct GitConfig {
    repository: String,
}

#[derive(Deserialize)]
struct FilesConfig {
    paths: Vec<String>,
}

impl Config {
    pub fn new(backup_folder: &str, config_path: &str, config_file: &str) -> Result<Self> {
        let backup_folder = expand_tilde_path(backup_folder)?;
        let mut config_path = expand_tilde_path(config_path)?;
        config_path.push(config_file);

        // Read and parse config file
        let config_str = fs::read_to_string(&config_path)?;
        let config: ConfigFile = toml::from_str(&config_str)
            .map_err(|e| SydError::ConfigParse(e.to_string()))?;

        Ok(Self {
            backup_folder,
            config_path,
            git_repo: config.git.repository,
        })
    }
}

pub struct Backup {
    config: Config,
    files: Vec<PathBuf>,
}

impl Backup {
    pub fn new(config: Config) -> Result<Self> {
        let files = read_config_files(&config.config_path)?;
        Ok(Self { config, files })
    }

    pub fn create_backup_folder(&self) -> Result<()> {
        if !self.config.backup_folder.exists() {
            std::fs::create_dir(&self.config.backup_folder)?;
            println!("Folder {:?} created.", self.config.backup_folder);
        } else {
            println!("Backup folder {:?} already exists", self.config.backup_folder);
        }
        Ok(())
    }

    pub fn backup_files(&self) -> Result<()> {
        for path in &self.files {
            let path = expand_tilde_path(path.to_str().unwrap())?;
            let file_name = path.file_name().unwrap();
            let dest = self.config.backup_folder.join(file_name);
            std::fs::copy(&path, &dest)?;
        }
        Ok(())
    }

    pub fn init_git_repo(&self) -> Result<()> {
        if !self.config.backup_folder.join(".git").exists() {
            git2::Repository::init(&self.config.backup_folder)?;
            println!("Git repository initialized in {:?}", self.config.backup_folder);
        } else {
            println!("Git repository already exists");
        }
        Ok(())
    }

    pub fn commit_and_push(&self) -> Result<()> {
        let repo = git2::Repository::open(&self.config.backup_folder)?;
        
        // Add all files
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        
        // Check if there are changes
        if repo.statuses(None)?.is_empty() {
            println!("All up to date. Nothing to commit and push.");
            return Ok(());
        }
        
        index.write()?;

        // Create commit
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        
        let sig = repo.signature()?;
        let message = "Update dotfiles";
        
        // Get HEAD or create initial commit
        let commit_id = if let Ok(head) = repo.head() {
            let parent = head.peel_to_commit()?;
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                message,
                &tree,
                &[&parent],
            )?
        } else {
            // Initial commit
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                message,
                &tree,
                &[],
            )?
        };

        // Create main branch if it doesn't exist
        if repo.find_branch("main", git2::BranchType::Local).is_err() {
            repo.branch("main", &repo.find_commit(commit_id)?, false)?;
        }

        // Set up remote with authentication
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            git2::Cred::ssh_key_from_agent("git")
        });

        // Remove existing remote if it exists
        if let Ok(mut remote) = repo.find_remote("origin") {
            remote.disconnect()?;
            repo.remote_delete("origin")?;
        }

        // Create new remote with authentication
        let mut remote = repo.remote_with_fetch("origin", &self.config.git_repo, "+refs/heads/*:refs/remotes/origin/*")?;
        
        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        // Push changes
        let refspec = "refs/heads/main:refs/heads/main";
        remote.push(&[refspec], Some(&mut push_opts))?;
        
        println!("Changes committed and pushed to remote");
        Ok(())
    }

    pub fn restore_files(&self) -> Result<()> {
        println!("Restoring files from {:?}", self.config.backup_folder);
        for path in &self.files {
            let file_name = path.file_name().unwrap();
            let source = self.config.backup_folder.join(file_name);
            
            // Expand destination path
            let dest = expand_tilde_path(path.to_str().unwrap())?;
            
            if source.exists() {
                // Create parent directories if they don't exist
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                
                std::fs::copy(&source, &dest)?;
                println!("Restored {:?} to {:?}", file_name, dest);
            } else {
                println!("Warning: Backup file {:?} not found", source);
            }
        }
        println!("Restore complete!");
        Ok(())
    }
}

fn expand_tilde_path(path: &str) -> Result<PathBuf> {
    simple_expand_tilde::expand_tilde(path)
        .ok_or_else(|| SydError::TildeExpansion(path.to_string()))
}

fn read_config_files(config_path: &PathBuf) -> Result<Vec<PathBuf>> {
    // Read and parse config file
    let config_str = fs::read_to_string(config_path)?;
    let config: ConfigFile = toml::from_str(&config_str)
        .map_err(|e| SydError::ConfigParse(e.to_string()))?;
    
    let files: Vec<PathBuf> = config.files.paths
        .into_iter()
        .map(PathBuf::from)
        .collect();
    
    println!("Config files to backup: {:?}", files);
    Ok(files)
} 