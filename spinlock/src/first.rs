use core::fmt;
use core::cell::UnsafeCell;
use core::fmt::Debug;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct SpinLock<T>{
    data: UnsafeCell<T>,
    flag: AtomicBool, 
}


impl<T> SpinLock<T>{

    #[inline(always)]
    pub const fn new(data: T) -> SpinLock<T>{
        
        SpinLock{
            data: UnsafeCell::new(data),
            flag: AtomicBool::new(false),
        }
    }
    
    #[must_use]
    pub fn lock(&self) -> SpinLockGuard<'_, T>{
        let mut backoff = 1u32;

        while self.flag.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            for _ in 0..backoff{
                core::hint::spin_loop();
            }
            if backoff < 1 << 10{
                backoff <<=1;
            }else{
                std::thread::yield_now();
            }
        }
        SpinLockGuard{
            lock: self,
        }
    }

    pub fn try_lock(&self) -> Result<SpinLockGuard<'_, T>, ()>{

        if self.flag.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            return Err(());
        }
        Ok(SpinLockGuard{
            lock: self,
        })
    }

    #[inline(always)]
    pub fn is_locked(&self) -> bool{
        self.flag.load(Ordering::Relaxed)
    }

    pub fn into_inner(self) -> T{
        self.data.into_inner()
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T{
        self.data.get_mut()
    }

    pub unsafe fn get_ptr(&self) -> *mut T{
        self.data.get()
    }

}




pub struct SpinLockGuard<'a, T>{
    lock: &'a SpinLock<T>,
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
        self.lock.flag.store(false, Ordering::Release);
    }
}

impl<T: Debug> Debug for SpinLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.try_lock() {
            Ok(g)  => f.debug_tuple("SpinLock").field(&*g).finish(),
            Err(_) => f.write_str("SpinLock(<locked>)"),
        }
    }
}

unsafe impl<T: Send> Send for SpinLock<T>{}
unsafe impl<T: Send> Sync for SpinLock<T>{}


