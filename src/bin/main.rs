use std::fs;

use diff_man::DiffManager;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[clap(subcommand)]
    pub mode: Mode,

    #[arg(short = 'd')]
    pub diff_path: std::path::PathBuf,

    #[arg(short = 't')]
    pub target_root: std::path::PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum Mode {
    #[command()]
    Apply,
    #[command()]
    Revert,
}

fn main() {
    let args = Args::parse();
    let diff_src =
        fs::read_to_string(args.diff_path).expect("cannot read diff src");
    let diffs =
        DiffManager::parse(&diff_man::diff::DiffFormat::GitUdiff, &diff_src)
            .expect("cannot parse given diff");
    match args.mode {
        Mode::Apply => diffs.apply(&args.target_root),
        Mode::Revert => diffs.revert(&args.target_root),
    }
    .expect("failed to execute");
}
