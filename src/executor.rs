use crate::signal::*;
use futures::future::BoxFuture;
use scoped_tls::scoped_thread_local;
scoped_thread_local!(static EX: Executor);

use std::{
    cell::RefCell,
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};

// scoped_thread_local!(static SIGNAL: Arc<Signal>);
// scoped_thread_local!(static RUNNABLE: Mutex<VecDeque<Arc<Task>>>);

struct ThreadPool {
    handles: Vec<std::thread::JoinHandle<()>>,
    sender: async_channel::Sender<Arc<Task>>,
}

impl ThreadPool {
    fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = async_channel::bounded(1024);
        let mut handles = Vec::with_capacity(size);
        for _ in 0..size {
            let recv: async_channel::Receiver<Arc<Task>> = receiver.clone();
            handles.push(std::thread::spawn(move || {
                while let Ok(task) = recv.try_recv() {
                    let waker = Waker::from(task.clone());
                    let mut cx = Context::from_waker(&waker);
                    let _ = task.future.borrow_mut().as_mut().poll(&mut cx);
                }
            }));
        }
        ThreadPool { handles, sender }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.handles.iter().for_each(|_| {
            let _ = self.sender.try_send(Arc::new(Task {
                future: RefCell::new(Box::pin(async {})),
                signal: Arc::new(Signal::new()),
            }));
        });
        self.handles.into_iter().for_each(|handle| {
            handle.join().unwrap();
        });
    }
}

pub struct Executor {
    pool: ThreadPool,
    runnable: Mutex<VecDeque<Arc<Task>>>,
}

impl Executor {
    pub fn new(size: usize) -> Executor {
        let pool = ThreadPool::new(size);
        let runnable = Mutex::new(VecDeque::new());
        Executor { pool, runnable }
    }

    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let task = Arc::new(Task {
            future: RefCell::new(Box::pin(future)),
            signal: Arc::new(Signal::new()),
        });
        self.runnable.lock().unwrap().push_back(task.clone());
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let mut fut = std::pin::pin!(future);
        let waker = Waker::from(Arc::new(Task {
            future: RefCell::new(Box::pin(async {})),
            signal: Arc::new(Signal::new()),
        }));
        let mut cx = Context::from_waker(&waker);
        EX.set(self, || loop {
            if let Poll::Ready(output) = fut.as_mut().poll(&mut cx) {
                return output;
            }
            while let Some(task) = self.runnable.lock().unwrap().pop_front() {
                self.pool.sender.try_send(task).unwrap();
            }
        })
    }
}
impl Default for Executor {
    fn default() -> Self {
        Self::new(1)
    }
}
struct Task {
    future: RefCell<BoxFuture<'static, ()>>,
    signal: Arc<Signal>,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        EX.with(|ex| {
            ex.runnable.lock().unwrap().push_back(self.clone());
        });
        self.signal.notify();
    }
}
