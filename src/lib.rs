use regex::Regex;
use sha1::{Digest, Sha1};
use std::sync::{mpsc::Sender, Arc, RwLock};

#[derive(Clone, Debug)]
pub struct ThreadInfo {
    pub hasher: Sha1,
    pub hashable: String,
    pub total_threads: u32,
    pub thread_offset: u32,
    pub author_timestamp: StringMatch,
    pub prefix: Prefix,
}

pub struct ChannelMessage {
    pub new_author_timestamp: u32,
    pub hash: String,
}

#[derive(Clone, Debug)]
pub struct StringMatch {
    pub value: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub struct Prefix {
    pub prefix: Vec<u8>,
    pub half_byte: bool,
}

impl Prefix {
    pub fn new(prefix: String) -> Self {
        let half_byte = prefix.len() % 2 != 0;
        let prefix = hex::decode(if half_byte { prefix + "0" } else { prefix }).unwrap();

        Self { prefix, half_byte }
    }

    pub fn is_start_of(&self, array: &[u8]) -> bool {
        let n = if self.half_byte {
            self.prefix.len() - 1
        } else {
            self.prefix.len()
        };

        if self.prefix[..n] != array[..n] {
            return false;
        }

        if self.half_byte {
            let byte = self.prefix[n];
            let masked_array = array[n] & 0xF0;
            return byte == masked_array;
        }

        true
    }
}

pub fn get_timestamps_from_last_commit(input: &String) -> (StringMatch, StringMatch) {
    let author_re = Regex::new(r"author .+? (\d+) .+").expect("Failed to create regex");
    let committer_re = Regex::new(r"committer .+? (\d+) .+").expect("Failed to create regex");

    let mut author_timestamp = None;
    let mut committer_timestamp = None;
    for line in input.lines() {
        if let Some(captures) = author_re.captures(&line) {
            if let Some(timestamp) = captures.get(1) {
                author_timestamp = Some(StringMatch {
                    value: timestamp.as_str().to_string(),
                    start: input.find(timestamp.as_str()).unwrap(),
                    end: input.find(timestamp.as_str()).unwrap() + timestamp.as_str().len(),
                });
            }
        }

        if let Some(captures) = committer_re.captures(&line) {
            if let Some(timestamp) = captures.get(1) {
                committer_timestamp = Some(StringMatch {
                    value: timestamp.as_str().to_string(),
                    start: input.find(timestamp.as_str()).unwrap(),
                    end: input.find(timestamp.as_str()).unwrap() + timestamp.as_str().len(),
                });
            }
        }
    }

    (
        author_timestamp.expect("No author timestamp found"),
        committer_timestamp.expect("No committer timestamp found"),
    )
}

pub fn calculate_threads(
    base_thread_info: ThreadInfo,
    done: Arc<RwLock<bool>>,
    tx: Sender<ChannelMessage>,
) -> Vec<std::thread::JoinHandle<()>> {
    let mut handles = vec![];

    for offset in 0..base_thread_info.total_threads {
        let mut new_thread = base_thread_info.clone();
        new_thread.thread_offset = offset;

        let done = done.clone();
        let tx = tx.clone();

        let handle =
            std::thread::spawn(move || calculate(new_thread, done, tx));

        handles.push(handle);
    }

    handles
}

pub fn calculate_sync(
    thread_info: ThreadInfo,
    done: Arc<RwLock<bool>>,
    tx: Sender<ChannelMessage>,
) -> Vec<std::thread::JoinHandle<()>> {
    calculate(thread_info, done, tx);

    Vec::new()
}

fn calculate(
    mut thread_info: ThreadInfo,
    done: Arc<RwLock<bool>>,
    tx: Sender<ChannelMessage>,
) {
    let mut new_author_timestamp = thread_info.author_timestamp.value.parse::<u32>().unwrap();
    new_author_timestamp -= thread_info.thread_offset;

    let mut author_timestamp_len= thread_info.author_timestamp.value.to_string().len();

    loop {
        if *done.read().unwrap() {
            return;
        }
        
        new_author_timestamp -= thread_info.total_threads;
        let new_author_timestamp_str = new_author_timestamp.to_string();

        if new_author_timestamp_str.len() < author_timestamp_len {
            println!("ono! only {} digits left!!!!!!1!!11", new_author_timestamp_str.len());
            thread_info.author_timestamp.end -= author_timestamp_len - new_author_timestamp_str.len();
            author_timestamp_len = new_author_timestamp.to_string().len();
        }
         
        thread_info.hashable.replace_range(
            &thread_info.author_timestamp.start..&thread_info.author_timestamp.end,
            &new_author_timestamp_str,
        );

        thread_info.hasher.update(&thread_info.hashable);
        let hash = &thread_info.hasher.finalize_reset();

        if thread_info.prefix.is_start_of(&hash.to_vec()) {
            tx.send(ChannelMessage {
                new_author_timestamp,
                hash: hex::encode(hash),
            })
            .unwrap();
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn calculate_rollover() {
        let hash_string = "author Test <test@example.com> 1000000000 +0200\ncommitter Test <test@example.com> 1000000000 +0200".to_string();
        let thread_info = ThreadInfo {
            author_timestamp: get_timestamps_from_last_commit(&hash_string).0,
            hasher: Sha1::new(),
            hashable: hash_string,
            total_threads: 1,
            thread_offset: 0,
            prefix: Prefix::new("0".to_string()),
        };

        let done = Arc::new(RwLock::new(false));
        let (tx, rx) = mpsc::channel();

        calculate(thread_info, done.clone(), tx.clone());
        let message = rx.recv().unwrap().hash;
        assert_eq!(message, "044fb45f2966662eb1d5b2eddd41ff023bdf4189");
    }
}
