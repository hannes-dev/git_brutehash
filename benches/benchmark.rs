use std::sync::mpsc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use lib::{calculate_sync, Prefix, StringMatch, ThreadInfo};
use sha1::{Digest, Sha1};

pub fn criterion_benchmark(c: &mut Criterion) {
    let input = ThreadInfo {
        hasher: Sha1::new(),
        hashable: "commit 173\0tree 2b297e643c551e76cfa1f93810c50811382f9117\nauthor Profile <profile@example.com> 1704063600 +0100\ncommitter Profile <profile@example.com> 1704063600 +0100\n\nprofile commit\n".to_string(),
        thread_num: 1,
        author_timestamp: StringMatch {
            value: "1704063600".to_string(),
            start: 94,
            end: 104,
        },
        prefix: Prefix{ prefix: vec![0x00, 0x00], half_byte: false },
    };

    let (tx, rx) = mpsc::channel();
    c.bench_with_input(
        BenchmarkId::new("create_thread", "profile"),
        &input,
        |b, input| {
            b.iter(|| {
                calculate_sync(
                    black_box(input.clone()),
                    black_box(Default::default()),
                    black_box(tx.clone()),
                )
            })
        },
    );

    drop(rx);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
