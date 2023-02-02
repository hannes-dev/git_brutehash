use clap::Parser;
use lib::{calculate_sync, calculate_threads, get_timestamps_from_last_commit, ThreadInfo};
use sha1::{Digest, Sha1};
use std::{
    process::Command,
    sync::{mpsc, Arc, RwLock},
};

#[derive(Parser, Debug)]
struct Args {
    /// hash prefix to search for
    #[arg(value_parser = hex_check)]
    prefix: String,
    /// search for hash but do not modify the commit
    #[arg(short, long, default_value = "false")]
    dry_run: bool,
    /// number of threads to use
    #[arg(short, long, default_value = "8", value_parser = clap::value_parser!(u32).range(1..=128))]
    threads: u32,
}

fn main() {
    let args = Args::parse();

    let cmd_output = Command::new("git")
        .args(["cat-file", "commit", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let output = String::from_utf8(cmd_output.stdout).expect("Failed to convert output to string");

    let (author_timestamp, committer_timestamp) = get_timestamps_from_last_commit(&output);

    let base_thread_info = ThreadInfo {
        hasher: Sha1::new(),
        hashable: format!("commit {}\0{}", output.len(), output),
        thread_num: args.threads,
        author_timestamp,
        prefix: args.prefix,
    };

    dbg!(&base_thread_info);

    let done = Arc::new(RwLock::new(false));
    let (tx, rx) = mpsc::channel();
    let handles = if args.threads == 1 {
        calculate_sync(base_thread_info, done.clone(), tx)
    } else {
        calculate_threads(base_thread_info, done.clone(), tx)
    };

    let message = rx.recv().unwrap();
    *done.write().unwrap() = true;

    println!("Found hash: {}", message.hash);

    if !args.dry_run {
        Command::new("git")
            .args([
                "commit",
                "--amend",
                "--allow-empty",
                "--no-edit",
                "--date",
                &(message.new_author_timestamp.to_string() + "+0200"),
            ])
            .env("GIT_COMMITTER_DATE", committer_timestamp)
            .output()
            .expect("Failed to execute git command");
        println!("Amended commit to hash {}", message.hash);
    } else {
        println!("Dry-run; no changes were made")
    }

    // cleanup the children
    for handle in handles {
        handle.join().unwrap();
    }
}

fn hex_check(s: &str) -> Result<String, String> {
    if s.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(s.to_string())
    } else {
        Err("Prefix must be a hex string".into())
    }
}
