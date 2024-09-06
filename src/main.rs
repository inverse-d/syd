use io::Error;
use simple_expand_tilde::*;
use std::fs::File;
use std::io::BufRead;
use std::path::PathBuf;
use std::{fs, io};

fn main() {
    let backup_folder_path = PathBuf::from("~/syd/");
    let config_path = "~/.config/syd/";
    let config_file = "syd.conf";
    let config = read_config_path(config_path.to_string(), config_file.to_string());
    println!("{:?}", read_config(config));
    create_backup_folder(backup_folder_path).unwrap()
}
fn read_config_path(config_path: String, config_file: String) -> PathBuf {
    let mut config = expand_tilde(config_path).expect("Failed to expand tilde into config path");
    config.push(PathBuf::from(config_file));
    config
}
fn read_config(config:PathBuf) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let file = File::open(config).expect("Could not open file");
    let reader = io::BufReader::new(file);
    for line in reader.lines() {
        let line = line.expect("Could not read line");
        paths.push(PathBuf::from(line));
    }
    paths
}
fn create_backup_folder(backup_folder_path: PathBuf) -> io::Result<()> {
    let expanded_path = expand_tilde(backup_folder_path).ok_or_else(|| {
        Error::new(
        io::ErrorKind::NotFound,
        "Failed to expand tilde"
        )})?;
    if !expanded_path.exists() {
        fs::create_dir(&expanded_path)?;
    } else {
        println!("Backup folder {:?} already exist", expanded_path)
    }
    Ok(())
}
fn backup_dotfiles() {}
fn restore_dotfiles() {}
fn create_local_repo() {}
fn push_to_git() {}
