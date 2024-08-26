# syd
A tool to back up and restore dotfiles. Originally it was started as a Golang project, but due to personal preference I switched it to Rust and began rewriting it in Rust. The original project can be found [here](https://github.com/inverse-d/syd_go)

## Functional requirements
- Support for config files
- Command line arguments
- Support for online repositories (GitHub, Gitlab)
- Written in Golang

## What it is supposed to do
To back up local dotfiles can be cumbersome, especially when they are not within `$HOME/.config/..` . Therefore, the idea is to write a tool which collects configured .dotfiles from all specified locations, writes them into a local git repository and pushes those to a cloud hosted repository like GitHub or Gitlab. In case of a restore one can also use the tool to put the files back to the places where they belong. The tool can be either configured via a .gitignore styled file, or it can be instructed with arguments. 


