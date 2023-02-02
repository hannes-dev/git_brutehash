use regex::Regex;
use sha1::{Digest, Sha1};
use std::sync::{mpsc::Sender, Arc, RwLock};

#[derive(Clone, Debug)]
pub struct ThreadInfo {
    pub hasher: Sha1,
    pub hashable: String,
    pub thread_num: u32,
    pub author_timestamp: String,
    pub prefix: String,
}

pub struct ChannelMessage {
    pub new_author_timestamp: u32,
    pub hash: String,
}

pub fn get_timestamps_from_last_commit(output: &String) -> (String, String) {
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

pub fn calculate_threads(
    base_thread_info: ThreadInfo,
    done: Arc<RwLock<bool>>,
    tx: Sender<ChannelMessage>,
) -> Vec<std::thread::JoinHandle<()>> {
    let mut handles = vec![];

    for offset in 0..base_thread_info.thread_num {
        let new_thread = base_thread_info.clone();
        let new_author_timestamp = new_thread.author_timestamp.parse::<u32>().unwrap() - offset;

        let done = done.clone();
        let tx = tx.clone();

        let handle =
            std::thread::spawn(move || calculate(new_thread, new_author_timestamp, done, tx));

        handles.push(handle);
    }

    handles
}

pub fn calculate_sync(
    thread_info: ThreadInfo,
    done: Arc<RwLock<bool>>,
    tx: Sender<ChannelMessage>,
) -> Vec<std::thread::JoinHandle<()>> {
    let new_author_timestamp = thread_info.author_timestamp.parse::<u32>().unwrap();
    calculate(thread_info, new_author_timestamp, done, tx);

    Vec::new()
}

fn calculate(
    mut thread_info: ThreadInfo,
    mut new_author_timestamp: u32,
    done: Arc<RwLock<bool>>,
    tx: Sender<ChannelMessage>,
) {
    loop {
        if *done.read().unwrap() {
            return;
        }

        new_author_timestamp -= thread_info.thread_num;

        let new_hashable = thread_info.hashable.replacen(
            &thread_info.author_timestamp,
            &new_author_timestamp.to_string(),
            1,
        );

        thread_info.hasher.update(&new_hashable);
        let hash = hex::encode(&thread_info.hasher.finalize_reset());

        if hash.starts_with(&thread_info.prefix) {
            tx.send(ChannelMessage {
                new_author_timestamp,
                hash,
            })
            .unwrap();
            return;
        }
    }
}
