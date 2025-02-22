use git2;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info, warn};
use serde::Deserialize;
use std::fs;
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
    #[error("Failed to parse config: {0}")]
    ConfigParse(String),
    #[error("Git config error: {0}")]
    GitConfig(String),
    #[error("Backup verification error: {0}")]
    BackupVerification(String),
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
    #[allow(dead_code)]
    commit_message: Option<String>,
    #[allow(dead_code)]
    branch: String,
}

#[derive(Deserialize)]
struct FilesConfig {
    paths: Vec<String>,
    #[allow(dead_code)]
    backup_dir: Option<String>,
    #[allow(dead_code)]
    ignore_patterns: Option<Vec<String>>,
}

impl Config {
    pub fn new(backup_folder: &str, config_path: &str, config_file: &str) -> Result<Self> {
        let backup_folder = expand_tilde_path(backup_folder)?;
        let mut config_path = expand_tilde_path(config_path)?;
        config_path.push(config_file);

        if !config_path.exists() {
            return Err(SydError::ConfigParse(format!(
                "Config file not found: {:?}",
                config_path
            )));
        }

        // Read and parse config file
        let config_str = fs::read_to_string(&config_path)?;
        let config: ConfigFile =
            toml::from_str(&config_str).map_err(|e| SydError::ConfigParse(e.to_string()))?;

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
            std::fs::create_dir_all(&self.config.backup_folder)?;
            info!("Folder {:?} created.", self.config.backup_folder);
        } else {
            info!(
                "Backup folder {:?} already exists",
                self.config.backup_folder
            );
        }
        Ok(())
    }

    fn file_needs_update(source: &PathBuf, dest: &PathBuf) -> Result<bool> {
        if !dest.exists() {
            return Ok(true);
        }

        let source_meta = fs::metadata(source)?;
        let dest_meta = fs::metadata(dest)?;

        Ok(source_meta.modified()? > dest_meta.modified()?)
    }

    pub fn backup_files(&self) -> Result<()> {
        self.create_backup_folder()?;
        self.init_git_repo()?;

        let mut changes = false;

        // First check what needs to be backed up
        for path in &self.files {
            let path = expand_tilde_path(path.to_str().unwrap())?;
            let dest = self.config.backup_folder.join(path.file_name().unwrap());

            if !dest.exists() || Self::file_needs_update(&path, &dest)? {
                changes = true;
                break;
            }
        }

        if !changes {
            info!("All files are up to date, no backup needed.");
            return Ok(());
        }

        let pb = ProgressBar::new(self.files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("##-"),
        );

        for path in &self.files {
            let path = expand_tilde_path(path.to_str().unwrap())?;
            let dest = self.config.backup_folder.join(path.file_name().unwrap());

            // Create parent directories if they don't exist
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::copy(&path, &dest)?;
            self.verify_backup(&path, &dest)?;
            pb.inc(1);
            pb.set_message(format!("Backing up {:?}", path));
        }

        pb.finish_with_message("Backup complete");

        self.commit_and_push()?;

        Ok(())
    }

    fn verify_backup(&self, source: &PathBuf, dest: &PathBuf) -> Result<()> {
        if !dest.exists() {
            return Err(SydError::BackupVerification(
                "Destination file not created".into(),
            ));
        }

        let source_meta = fs::metadata(source)?;
        let dest_meta = fs::metadata(dest)?;

        if source_meta.len() != dest_meta.len() {
            return Err(SydError::BackupVerification(
                "File sizes don't match".into(),
            ));
        }

        Ok(())
    }

    fn ensure_git_configured(&self, repo: &git2::Repository) -> Result<()> {
        if repo.config()?.get_string("user.name").is_err() {
            return Err(SydError::GitConfig("Git user.name not configured".into()));
        }
        if repo.config()?.get_string("user.email").is_err() {
            return Err(SydError::GitConfig("Git user.email not configured".into()));
        }
        Ok(())
    }

    pub fn init_git_repo(&self) -> Result<()> {
        if !self.config.backup_folder.join(".git").exists() {
            git2::Repository::init(&self.config.backup_folder)?;
            info!(
                "Git repository initialized in {:?}",
                self.config.backup_folder
            );
        } else {
            info!("Git repository already exists");
        }
        Ok(())
    }

    pub fn commit_and_push(&self) -> Result<()> {
        let repo = git2::Repository::open(&self.config.backup_folder)?;
        self.ensure_git_configured(&repo)?;

        // Add all files
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Check if there are changes
        if repo.statuses(None)?.is_empty() {
            info!("All up to date. Nothing to commit and push.");
            return Ok(());
        }

        // Create commit
        let tree_id = index.write_tree()?;
        let _tree = repo.find_tree(tree_id)?;

        let _sig = repo.signature()?;
        let _message = "Update dotfiles";

        // Get HEAD or create initial commit
        let commit_id = if let Ok(head) = repo.head() {
            let _parent = head.peel_to_commit()?;
            repo.commit(Some("HEAD"), &_sig, &_sig, _message, &_tree, &[&_parent])?
        } else {
            // Initial commit
            repo.commit(Some("HEAD"), &_sig, &_sig, _message, &_tree, &[])?
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
        let mut remote = repo.remote_with_fetch(
            "origin",
            &self.config.git_repo,
            "+refs/heads/*:refs/remotes/origin/*",
        )?;

        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        // Push changes
        let refspec = "refs/heads/main:refs/heads/main";
        remote.push(&[refspec], Some(&mut push_opts))?;

        info!("Changes committed and pushed to remote");
        Ok(())
    }

    pub fn pull_from_remote(&self) -> Result<()> {
        let repo = git2::Repository::open(&self.config.backup_folder)?;
        self.ensure_git_configured(&repo)?;

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            git2::Cred::ssh_key_from_agent("git")
        });

        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        // Fetch from remote
        let mut remote = repo.find_remote("origin")?;
        remote.fetch(&["main"], Some(&mut fetch_opts), None)?;

        // Get remote main branch
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = fetch_head.peel_to_commit()?;

        // Set local main branch to remote
        let mut reference = repo.find_reference("refs/heads/main")?;
        reference.set_target(fetch_commit.id(), "Fast-forward update")?;

        info!("Successfully pulled latest changes from remote");
        Ok(())
    }

    pub fn clone_or_pull(&self) -> Result<()> {
        if !self.config.backup_folder.join(".git").exists() {
            info!("Cloning repository from remote...");
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(|_url, _username_from_url, _allowed_types| {
                git2::Cred::ssh_key_from_agent("git")
            });

            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);

            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fetch_opts);

            builder.clone(&self.config.git_repo, &self.config.backup_folder)?;
            info!("Repository cloned successfully");
        } else {
            self.pull_from_remote()?;
        }
        Ok(())
    }

    pub fn restore_files(&self) -> Result<()> {
        // Clone or pull before restoring
        self.clone_or_pull()?;

        let mut changes = false;

        // First check what needs to be restored
        for path in &self.files {
            let file_name = path.file_name().unwrap();
            let source = self.config.backup_folder.join(file_name);
            let dest = expand_tilde_path(path.to_str().unwrap())?;

            if !dest.exists() || Self::file_needs_update(&source, &dest)? {
                changes = true;
                break;
            }
        }

        if !changes {
            info!("All files are up to date, no restore needed.");
            return Ok(());
        }

        info!("Restoring files from {:?}", self.config.backup_folder);
        let pb = ProgressBar::new(self.files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("##-"),
        );

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
                self.verify_backup(&source, &dest)?;
                pb.inc(1);
                pb.set_message(format!("Restored {:?} to {:?}", file_name, dest));
            } else {
                warn!("Warning: Backup file {:?} not found", source);
            }
        }
        pb.finish_with_message("Restore complete!");
        Ok(())
    }

    pub fn status(&self) -> Result<()> {
        println!("\n=== Syd Backup Status ===\n");

        // Check backup folder
        println!("Backup Directory: {:?}", self.config.backup_folder);
        if self.config.backup_folder.exists() {
            println!("Status: ✓ Exists");
        } else {
            println!("Status: ✗ Does not exist");
        }

        // Git status
        println!("\nGit Repository:");
        println!("Remote URL: {}", self.config.git_repo);

        if let Ok(repo) = git2::Repository::open(&self.config.backup_folder) {
            let statuses = repo.statuses(None)?;

            if let Ok(head) = repo.head() {
                if let Some(branch_name) = head.shorthand() {
                    println!("Current Branch: {}", branch_name);
                }
            }

            let mut modified = 0;
            let mut untracked = 0;

            for entry in statuses.iter() {
                match entry.status() {
                    s if s.is_wt_modified() => modified += 1,
                    s if s.is_wt_new() => untracked += 1,
                    _ => {}
                }
            }

            println!("Modified files: {}", modified);
            println!("Untracked files: {}", untracked);
        } else {
            println!("Status: ✗ Not initialized");
        }

        println!("\nTracked Files:");
        for path in &self.files {
            let path = expand_tilde_path(path.to_str().unwrap())?;
            let file_name = path.file_name().unwrap();
            let backup_path = self.config.backup_folder.join(file_name);

            print!("{:?}: ", path);
            if path.exists() {
                if backup_path.exists() {
                    let orig_modified = path.metadata()?.modified()?;
                    let backup_modified = backup_path.metadata()?.modified()?;

                    if orig_modified > backup_modified {
                        println!("⚠ Local file newer than backup");
                    } else {
                        println!("✓ Synced");
                    }
                } else {
                    println!("✗ Not backed up");
                }
            } else {
                println!("✗ Source file missing");
            }
        }

        println!();
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
    let config: ConfigFile =
        toml::from_str(&config_str).map_err(|e| SydError::ConfigParse(e.to_string()))?;

    let files: Vec<PathBuf> = config.files.paths.into_iter().map(PathBuf::from).collect();

    info!("Config files to backup: {:?}", files);
    Ok(files)
}
