use regex::Regex;
use sha1::{Digest, Sha1};
use std::{
    env, ops::Not, process::Command, sync::{
        mpsc::{self, Sender},
        Arc, RwLock,
    }
};

#[derive(Clone)]
struct ThreadInfo {
    hasher: Sha1,
    hashable: String,
    thread_num: u32,
    author_timestamp: String,
    prefix: String,
}

fn main() {
    let prefix = env::args().nth(1).expect("Please provide a prefix");
    if prefix.chars().all(|c| c.is_ascii_hexdigit()).not() {
        panic!("Prefix must be a hex string");
    }

    let thread_num: u32 = env::args()
        .nth(2)
        .unwrap_or("8".into())
        .parse()
        .expect("Please provide a number of threads (int)");

    let cmd_output = Command::new("git")
        .args(["cat-file", "commit", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let output = String::from_utf8(cmd_output.stdout).expect("Failed to convert output to string");

    let (author_timestamp, committer_timestamp) = get_timestamps_from_last_commit(&output);

    let base_thread_info = ThreadInfo {
        hasher: Sha1::new(),
        hashable: format!("commit {}\0{}", output.len(), output),
        thread_num,
        author_timestamp,
        prefix,
    };

    let done = Arc::new(RwLock::new(false));
    let (tx, rx) = mpsc::channel();
    let handles = create_threads(base_thread_info, done.clone(), tx);

    let new_author_timestamp = rx.recv().unwrap();
    *done.write().unwrap() = true;

    Command::new("git")
        .args(["commit", "--amend", "--allow-empty", "--no-edit", "--date", &(new_author_timestamp.to_string() + "+0200")])
        .env("GIT_COMMITTER_DATE", committer_timestamp)
        .output()
        .expect("Failed to execute git command");

    // cleanup the children
    for handle in handles {
        handle.join().unwrap();
    }

    println!("Done!");
}

fn get_timestamps_from_last_commit(output: &String) -> (String, String) {
    let author_re = Regex::new(r"author .+? (\d+) .+").expect("Failed to create regex");
    let committer_re = Regex::new(r"committer .+? (\d+) .+").expect("Failed to create regex");

    let mut author_timestamp = "";
    let mut committer_timestamp = "";
    for line in output.lines() {
        if let Some(captures) = author_re.captures(&line) {
            if let Some(timestamp) = captures.get(1) {
                author_timestamp = timestamp.as_str();
            }
        }

        if let Some(captures) = committer_re.captures(&line) {
            if let Some(timestamp) = captures.get(1) {
                committer_timestamp = timestamp.as_str();
            }
        }
    }

    (
        author_timestamp.to_string(),
        committer_timestamp.to_string(),
    )
}

fn create_threads(
    base_thread_info: ThreadInfo,
    done: Arc<RwLock<bool>>,
    tx: Sender<u32>,
) -> Vec<std::thread::JoinHandle<()>> {
    let mut handles = vec![];

    for offset in 0..base_thread_info.thread_num {
        let mut new_thread = base_thread_info.clone();

        let mut new_author_timestamp = new_thread.author_timestamp.parse::<u32>().unwrap() - offset;

        let done = done.clone();
        let tx = tx.clone();

        let handle = std::thread::spawn(move || loop {
            if *done.read().unwrap() {
                return;
            }

            new_author_timestamp -= new_thread.thread_num;

            let new_hashable = new_thread
                .hashable
                .replacen(
                    &new_thread.author_timestamp,
                    &new_author_timestamp.to_string(),
                    1
                );

            new_thread.hasher.update(&new_hashable);
            let hash = hex::encode(&new_thread.hasher.finalize_reset());

            if hash.starts_with(&new_thread.prefix) {
                dbg!(new_hashable);
                tx.send(new_author_timestamp).unwrap();
                return;
            }
        });

        handles.push(handle);
    }

    handles
}
