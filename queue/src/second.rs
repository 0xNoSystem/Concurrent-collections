use core::cell::RefCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use crossbeam::epoch::{self, Atomic, Guard, Owned, Shared};
use crossbeam::utils::CachePadded;
use std::sync::atomic::Ordering;

thread_local! {
    static THREAD_GUARD: RefCell<Option<Guard>> = const{RefCell::new(None)};
}

#[repr(align(64))]
pub struct AtomicQueue<T> {
    head: CachePadded<Link<T>>,
    tail: CachePadded<Link<T>>,

    _marker: PhantomData<T>,
}

type Link<T> = Atomic<Node<T>>;
struct Node<T> {
    data: MaybeUninit<T>,
    next: Link<T>,
}

impl<T> Node<T> {
    fn dummy() -> Self {
        Node {
            data: MaybeUninit::uninit(),
            next: Atomic::null(),
        }
    }
}

impl<T> AtomicQueue<T> {
    pub fn new() -> Self {
        let dummy = CachePadded::new(Atomic::new(Node::dummy()));
        AtomicQueue {
            head: dummy.clone(),
            tail: dummy,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        THREAD_GUARD.with(|cell| {
            let mut guard_ref = cell.borrow_mut();

            if guard_ref.is_none() {
                *guard_ref = Some(epoch::pin());
            }
            let g = guard_ref.as_ref().unwrap();
            let head = self.head.load(Ordering::Acquire, g);
            let next = unsafe { head.as_ref().unwrap().next.load(Ordering::Acquire, g) };
            next.is_null()
        })
    }

    #[inline(always)]
    pub fn enqueue(&self, data: T) {
        THREAD_GUARD.with(|cell| {
            let mut guard_ref = cell.borrow_mut();

            if guard_ref.is_none() {
                *guard_ref = Some(epoch::pin());
            }
            let g = guard_ref.as_ref().unwrap();
            let new_node = Owned::new(Node {
                data: MaybeUninit::new(data),
                next: Atomic::null(),
            })
            .into_shared(g);

            loop {
                let tail = self.tail.load(Ordering::Acquire, g);
                let tail_ref = unsafe { tail.as_ref().unwrap() };
                let next = tail_ref.next.load(Ordering::Acquire, g);

                if next.is_null() {
                    let success = tail_ref
                        .next
                        .compare_exchange(next, new_node, Ordering::Release, Ordering::Relaxed, g)
                        .is_ok();

                    if success {
                        //advance the tail, tail might still be lagging
                        let _ = self.tail.compare_exchange(
                            tail,
                            new_node,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                            g,
                        );
                        break;
                    }
                } else {
                    //tail is lagging, advance
                    let _ = self.tail.compare_exchange(
                        tail,
                        next,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                        g,
                    );
                }
            }
        })
    }

    #[inline(always)]
    pub fn dequeue(&self) -> Option<T> {
        THREAD_GUARD.with(|cell| {
            let mut guard_ref = cell.borrow_mut();

            if guard_ref.is_none() {
                *guard_ref = Some(epoch::pin());
            }
            let g = guard_ref.as_ref().unwrap();
            loop {
                let head = self.head.load(Ordering::Acquire, g);
                let head_ref = unsafe { head.as_ref().unwrap() };

                let next = head_ref.next.load(Ordering::Acquire, g);
                if next.is_null() {
                    return None;
                }
                if self
                    .head
                    .compare_exchange(head, next, Ordering::Release, Ordering::Relaxed, g)
                    .is_ok()
                {
                    //drop previous dummy
                    let val = Self::take_data(next);
                    unsafe { g.defer_destroy(head) };
                    return Some(val);
                }
            }
        })
    }

    #[inline(always)]
    fn take_data(ptr: Shared<'_, Node<T>>) -> T {
        unsafe { ptr.as_ref().unwrap().data.assume_init_read() }
    }
}

unsafe impl<T: Send> Sync for AtomicQueue<T> {}
unsafe impl<T: Send> Send for AtomicQueue<T> {}

impl<T> Default for AtomicQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for AtomicQueue<T> {
    fn drop(&mut self) {
        let g = &epoch::pin();
        let mut head = self.head.load(Ordering::Acquire, g);

        let mut c = 0;
        while !head.is_null() {
            unsafe {
                let node_ref = head.deref_mut();
                let next = node_ref.next.load(Ordering::Acquire, g);

                if c > 0 {
                    node_ref.data.assume_init_drop();
                }

                g.defer_destroy(head);
                head = next;
                c += 1;
            }
        }
    }
}

