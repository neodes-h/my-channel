use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.sender += 1;
        drop(inner);
        Sender {
            // use this
            shared: Arc::clone(&self.shared),
            // don't use this
            // shared: self.shared.clone()
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.sender -= 1;
        let was_last = inner.sender == 0;
        drop(inner);
        if was_last {
            // For one case:
            // Receiver went to sleep
            // Last sender is dropped
            // Then the receiver will never be waken up
            self.shared.available.notify_one();
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&mut self, t: T) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.queue.push_back(t);
        // This is CRUCIAL.
        // If the lock is not dropped, the receiver end will never acquire the lock when it is
        // waken up.
        drop(inner);
        self.shared.available.notify_one();
    }
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Option<T> {
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            match inner.queue.pop_front() {
                Some(t) => return Some(t),
                None if inner.sender < 1 => return None,
                // if queue is empty, put this thread to sleep until it is waken up by the sender
                // thread using Condvar.
                // Once it is awaken, it will acquire the MutexLock immediately.
                // Before going to sleep, the lock **queue** has to be taken ownership
                // otherwise the other thread will never acquire the lock
                None => inner = self.shared.available.wait(inner).unwrap(),
            }
        }
    }
}

struct Inner<T> {
    queue: VecDeque<T>,
    sender: usize,
}

struct Shared<T> {
    inner: Mutex<Inner<T>>,
    available: Condvar,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Inner {
        queue: VecDeque::default(),
        sender: 1,
    };
    let shared = Shared {
        inner: Mutex::new(inner),
        available: Condvar::new(),
    };
    let shared = Arc::new(shared);
    (
        Sender {
            shared: shared.clone(),
        },
        Receiver {
            shared: shared.clone(),
        },
    )
}

#[cfg(test)]
mod test {
    use super::{channel, Receiver, Sender};

    #[test]
    fn ping_pong() {
        let (mut sender, mut receiver) = channel();
        sender.send(3);
        assert_eq!(receiver.recv(), Some(3));
    }

    #[test]
    fn close() {
        let (sender, mut receiver) = channel::<()>();
        drop(sender);
        assert_eq!(receiver.recv(), None);
    }
}
