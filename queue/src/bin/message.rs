use std::thread::{self, ThreadId};
use queue::AtomicQueue;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Debug)]
struct Message{
    data: String,
    sender: ThreadId,
}

impl Message{
    #[inline(always)]
    fn new(data: String) -> Self {
        Message {
            data,
            sender: thread::current().id(),
        }
    }
}


fn main(){
    let (tx, rx) = Channel::new();
    let tx1 = tx.clone();
    let tx2 = tx.clone();

    let h1 = thread::spawn(move||{
        let mut c = 0;
        loop{
            let _ = thread::sleep(Duration::from_millis(300));
            let msg = format!("This is message number {}",c);
            tx.send(msg);
            c+= 1;
        }
    });
    let h2 = thread::spawn(move||{
        let mut c = 0;
        loop{
            let _ = thread::sleep(Duration::from_millis(400));
            let msg = format!("This is message number {}",c*2);
            tx1.send(msg);
            c+= 1;
        }
    });

    let h3 = thread::spawn(move||{
        let mut c = 0;
        loop{
            let _ = thread::sleep(Duration::from_millis(500));
            let msg = format!("This is message number {}",c*3);
            tx2.send(msg);
            c+= 1;
        }
    });
    loop{
        if let Some(msg) = rx.recv(){
            println!("{:?}", msg);
        }
    }
    

    
}




struct Channel<T>{
    sender: Sender<T>,
    receiver: Receiver<T>,
}

#[derive(Clone)]
struct Sender<T>{
    sender: Arc<AtomicQueue<T>>,
}

impl<T> Sender<T>{

    fn send(&self, msg: T){
        let e = self.sender.enqueue(msg);
    }
}


struct Receiver<T>{
    receiver: Arc<AtomicQueue<T>>,
}

impl<T> Receiver<T>{
    fn recv(&self) -> Option<T>{
        self.receiver.dequeue()
    }
}

impl<T> Channel<T>{
    fn new() -> (Sender<T>, Receiver<T>){
        let q: Arc<AtomicQueue<T>> = Arc::new(AtomicQueue::new());
        
        (Sender{sender: q.clone()}, Receiver{receiver: q})
    }

}
