use simple_expand_tilde::*;
use std::path::PathBuf;

fn main() {
    let config_path = "~/.config/";
    let config_file = "syd.conf";
    println!("{:?}", read_config(config_path.to_string(), config_file.to_string()))
}
fn read_config(
    config_path: String,
    config_file: String
) -> PathBuf {
    if config_path.contains("~") {
        let mut config = expand_tilde(config_path).unwrap();
        config.push(PathBuf::from(config_file));
        config
    } else {
        let mut config = PathBuf::from(config_path);
        config.push(PathBuf::from(config_file));
        config
    }
}
fn create_backup_folder() {}
fn backup_dotfiles() {}
fn restore_dotfiles() {}
fn create_local_repo() {}
fn push_to_git() {}
