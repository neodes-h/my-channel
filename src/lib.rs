use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            // use this
            inner: Arc::clone(&self.inner),
            // don't use this
            // inner: self.inner.clone()
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&mut self, t: T) {
        let mut queue = self.inner.queue.lock().unwrap();
        queue.push_back(t);
        // This is CRUCIAL.
        // If the lock is not dropped, the receiver end will never acquire the lock when it is
        // waken up.
        drop(queue);
        self.inner.available.notify_one();
    }
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> T {
        let mut queue = self.inner.queue.lock().unwrap();
        loop {
            match queue.pop_front() {
                Some(t) => return t,
                // if queue is empty, put this thread to sleep until it is waken up by the sender
                // thread using Condvar.
                // Once it is awaken, it will acquire the MutexLock immediately.
                // Before going to sleep, the lock **queue** has to be taken ownership
                // otherwise the other thread will never acquire the lock
                None => queue = self.inner.available.wait(queue).unwrap(),
            }
        }
    }
}

struct Inner<T> {
    queue: Mutex<VecDeque<T>>,
    available: Condvar,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Inner {
        queue: Mutex::default(),
        available: Condvar::new(),
    };
    let inner = Arc::new(inner);
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver {
            inner: inner.clone(),
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
        assert_eq!(receiver.recv(), 3);
    }
}
