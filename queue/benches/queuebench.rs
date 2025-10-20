use criterion::{Criterion, criterion_group, criterion_main};
use crossbeam::queue::SegQueue;
use queue::AtomicQueue;
use std::sync::Arc;
use std::thread;

fn bench_single_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_thread");

    group.bench_function("AtomicQueue single-thread", |b| {
        b.iter(|| {
            let q = AtomicQueue::new();
            for i in 0..1_000_0 {
                q.enqueue(i);
                q.dequeue();
            }
        });
    });

    group.bench_function("SegQueue single-thread", |b| {
        b.iter(|| {
            let q = SegQueue::new();
            for i in 0..1_000_0 {
                q.push(i);
                q.pop();
            }
        });
    });

    group.finish();
}

fn bench_multi_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_thread");
    let thread_counts = [2, 4, 8, 16];
    let iters = 100_000;

    for &threads in &thread_counts {
        group.bench_function(format!("AtomicQueue {} threads", threads), |b| {
            b.iter(|| {
                let q = Arc::new(AtomicQueue::new());
                let handles: Vec<_> = (0..threads)
                    .map(|_| {
                        let q = q.clone();
                        thread::spawn(move || {
                            for i in 0..iters {
                                q.enqueue(i);
                                q.dequeue();
                            }
                        })
                    })
                    .collect();

                for h in handles {
                    h.join().unwrap();
                }
            });
        });

        group.bench_function(format!("SegQueue {} threads", threads), |b| {
            b.iter(|| {
                let q = Arc::new(SegQueue::new());
                let handles: Vec<_> = (0..threads)
                    .map(|_| {
                        let q = q.clone();
                        thread::spawn(move || {
                            for i in 0..iters {
                                q.push(i);
                                q.pop();
                            }
                        })
                    })
                    .collect();

                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_single_thread, bench_multi_thread);
criterion_main!(benches);
