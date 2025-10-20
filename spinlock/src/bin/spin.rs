use spinlock::second::SpinLock;
use std::thread;
use std::sync::Arc;


fn main() {
    const NUM_THREADS: usize = 5;
    const INCREMENTS: usize = 100_000;

    let lock = Arc::new(SpinLock::new(0usize));

    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let lock_clone = Arc::clone(&lock);
        let handle = thread::spawn(move || {
            for _ in 0..INCREMENTS {
                let mut guard = lock_clone.lock();
                *guard += 1;
                // guard drops here -> unlock
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let result = *lock.lock();
    println!("Final counter value: {}", result);

    assert_eq!(result, NUM_THREADS * INCREMENTS);
    println!("Test passed ✅");

}
