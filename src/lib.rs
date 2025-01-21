use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SydError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to expand tilde: {0}")]
    TildeExpansion(String),
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
}

pub type Result<T> = std::result::Result<T, SydError>;

pub struct Config {
    backup_folder: PathBuf,
    config_path: PathBuf,
}

impl Config {
    pub fn new(backup_folder: &str, config_path: &str, config_file: &str) -> Result<Self> {
        let backup_folder = expand_tilde_path(backup_folder)?;
        let mut config_path = expand_tilde_path(config_path)?;
        config_path.push(config_file);

        Ok(Self {
            backup_folder,
            config_path,
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

    pub fn commit_and_push(&self, remote_url: &str) -> Result<()> {
        let repo = git2::Repository::open(&self.config.backup_folder)?;
        
        // Add all files
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
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
        let mut remote = repo.remote_with_fetch("origin", remote_url, "+refs/heads/*:refs/remotes/origin/*")?;
        
        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        // Push changes
        let refspec = "refs/heads/main:refs/heads/main";
        remote.push(&[refspec], Some(&mut push_opts))?;
        
        println!("Changes committed and pushed to remote");
        Ok(())
    }
}

fn expand_tilde_path(path: &str) -> Result<PathBuf> {
    simple_expand_tilde::expand_tilde(path)
        .ok_or_else(|| SydError::TildeExpansion(path.to_string()))
}

fn read_config_files(config_path: &PathBuf) -> Result<Vec<PathBuf>> {
    let file = std::fs::File::open(config_path)?;
    let reader = std::io::BufReader::new(file);
    let files: Vec<PathBuf> = std::io::BufRead::lines(reader)
        .filter_map(|line| line.ok())
        .map(PathBuf::from)
        .collect();
    
    println!("Config files to backup: {:?}", files);
    Ok(files)
} 