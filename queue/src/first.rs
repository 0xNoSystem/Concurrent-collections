use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use core::cell::UnsafeCell;
use core::mem::{MaybeUninit};

#[repr(align(64))]
pub struct AtomicQueue<T>{
    head: Link<T>,
    tail: Link<T>,
}

type Link<T> = AtomicPtr<UnsafeCell<Node<T>>>;


pub struct Node<T>{
    data: T,
    next: Link<T>,
}

impl<T> Node<T>{
fn dummy() -> *mut UnsafeCell<Node<T>>{
    Box::into_raw(Box::new(
        UnsafeCell::new(Node{
        data: unsafe { core::mem::zeroed() },
        next: null_atomic_ptr(),
        }
        )))
    }

}

impl<T> AtomicQueue<T>{

    pub fn new() -> Self{
            let node = Node::dummy();
            AtomicQueue{
            head: AtomicPtr::new(node),
            tail: AtomicPtr::new(node),
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool{
        unsafe{
            (*(*self.head.load(Ordering::Acquire)).get()).next.load(Ordering::Acquire).is_null()
            
        }
    }

    pub fn enqueue(&self, data: T){
        //add to tail
        let new_node = Box::into_raw(Box::new(
                UnsafeCell::new(Node{
                    data,
                    next: null_atomic_ptr(),
                }
            )));

        loop{
            let tail = self.tail.load(Ordering::Acquire);
            let next = unsafe{(*(*tail).get()).next.load(Ordering::Acquire)};

            if next.is_null(){

                let success = unsafe{
                    (*(*tail).get()).next.compare_exchange(
                        ptr::null_mut(),
                        new_node,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    ).is_ok()
                };
                
                if success{
                    let _ = self.tail.compare_exchange(
                        tail,
                        new_node,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                    break;
                }

            }else{
                let _ = self.tail.compare_exchange(
                    tail,
                    next,
                    Ordering::Release,
                    Ordering::Relaxed,
                );
            }
        }
    }


    /*
        head -> dummy -> [n1] -> [n2]
                                   ^
                                   |
                                  tail
    */

    pub fn dequeue(&self) -> Option<T>{
        //remove from head
        unsafe{
            loop{
                let head = self.head.load(Ordering::Acquire);
                let next = (*(*head).get()).next.load(Ordering::Acquire);
                if next.is_null(){
                    return None;        
                }

                if self.head.compare_exchange(
                        head,
                        next,
                        Ordering::Release,
                        Ordering::Relaxed,
                ).is_ok(){
                    let _ = Box::from_raw(head);
                    return Some(Self::take_data(next)); 
                }
            }
        }
    } 

    
        #[inline(always)]
            fn take_data(ptr: *mut UnsafeCell<Node<T>>) -> T {
        unsafe {
            let b = Box::from_raw((*ptr).get());            
            let mut m: MaybeUninit<T> = MaybeUninit::uninit();
            let data = core::mem::replace(&mut *m.as_mut_ptr() , b.data);
            
            data
        // Drop boxed_cell here, Node is dropped (except data, which we moved)
        //THIS IS UB    
        }
        }

}

unsafe impl<T: Send> Sync for AtomicQueue<T> {}
unsafe impl<T: Send> Send for AtomicQueue<T> {}



#[inline(always)]
fn null_atomic_ptr<T>() -> AtomicPtr<T>{
    AtomicPtr::new(ptr::null_mut())
}





