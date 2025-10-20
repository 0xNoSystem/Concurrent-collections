use core::fmt;
use core::cell::UnsafeCell;
use core::fmt::Debug;
use core::sync::atomic::{AtomicBool,AtomicPtr, Ordering};
use std::thread;
use std::thread::{Thread};

pub struct SpinLock<T>{
    data: UnsafeCell<T>,
    tail: Link,//track callers, to make lock access fair,
}

type Link = AtomicPtr<Node>;

struct Node{
    handle: Thread,
    next: Link,
    locked: AtomicBool,
}


impl<T> SpinLock<T>{

    #[inline(always)]
    pub fn new(data: T) -> Self{
        
        SpinLock{
            data: UnsafeCell::new(data),
            tail: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    #[must_use]
    pub fn lock(&self) -> SpinLockGuard<'_, T>{
        let node = Node{
                    handle: thread::current(),
                    next: AtomicPtr::new(core::ptr::null_mut()),
                    locked: AtomicBool::new(true),
            };
        let boxed_new = Box::into_raw(Box::new(node));
        let node_ptr = self.tail.swap(boxed_new, Ordering::Release);

        if !node_ptr.is_null(){
                unsafe{
                    (*node_ptr).next.store(boxed_new, Ordering::Release);
                    let mut s: u8 = 0b0110_0100;
                    while (*boxed_new).locked.load(Ordering::Acquire){
                        if s > 0{
                            core::hint::spin_loop();
                            s -= 1;
                        }
                        thread::park();
                        core::hint::spin_loop();
                    }
                };
            }else{
                unsafe{
                    (*boxed_new).locked.store(false, Ordering::Release);
                }
            }

        println!("Thread {:?} acquired the lock", thread::current().id());
        SpinLockGuard{
            lock: self,
            node: boxed_new, 
        }
    }

    pub fn try_lock(&self) -> Result<SpinLockGuard<'_, T>, ()>{
        let node = Box::into_raw(Box::new(Node{
            handle: thread::current(),
            locked: false.into(),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }));

        match self.tail.compare_exchange(
            core::ptr::null_mut(),
            node,
            Ordering::AcqRel,
            Ordering::Relaxed,
        )
        {
            Ok(_) => Ok(SpinLockGuard{lock: self,node }),
            Err(_) => {
                unsafe{
                    drop(Box::from_raw(node));
                }
                Err(())
            }
        }

    }

     

}








pub struct SpinLockGuard<'a, T>{
    lock: &'a SpinLock<T>,
    node: *mut Node,
}

impl<'a, T> SpinLockGuard<'a, T>{
    fn unlock(&mut self){
        unsafe{
            let next = (*self.node).next.load(Ordering::Acquire);
            if next.is_null(){
                if self.lock.tail.compare_exchange(
                    self.node,
                    core::ptr::null_mut(),
                    Ordering::Release,
                    Ordering::Relaxed,
                ).is_ok(){
                    (*self.node).locked.store(true, Ordering::Release);
                }else{
                    while (*self.node).next.load(Ordering::Acquire).is_null(){
                        core::hint::spin_loop();
                    }
                    let next = (*self.node).next.load(Ordering::Acquire);
                    (*next).locked.store(false, Ordering::Release);
                    (*next).handle.unpark();
                }
            }else{
                (*next).locked.store(false, Ordering::Relaxed);
                (*next).handle.unpark();
            }
        }

    }

}


impl<'a, T> std::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> std::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        unsafe{
            self.unlock();
            drop(Box::from_raw(self.node));
        }
    }
}


unsafe impl<T: Send> Send for SpinLock<T>{}
unsafe impl<T: Send> Sync for SpinLock<T>{}


