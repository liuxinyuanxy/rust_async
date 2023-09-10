use crate::signal::*;
use async_channel::TryRecvError;
use futures::future::BoxFuture;
use scoped_tls::scoped_thread_local;
scoped_thread_local!(pub(crate) static EX: Executor);

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
    handles: Option<Vec<std::thread::JoinHandle<()>>>, // according to https://stackoverflow.com/questions/63756181/cannot-move-out-of-handle-which-is-behind-a-shared-reference-move-occurs-beca
    sender: async_channel::Sender<Arc<Task>>,
}

impl ThreadPool {
    fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = async_channel::bounded(1024);
        let mut handles = Vec::with_capacity(size);
        for _ in 0..size {
            let recv: async_channel::Receiver<Arc<Task>> = receiver.clone();
            handles.push(std::thread::spawn(move || loop {
                match recv.try_recv() {
                    Ok(task) => {
                        let waker = Waker::from(task.clone());
                        let mut cx = Context::from_waker(&waker);
                        let _ = task.future.borrow_mut().as_mut().poll(&mut cx);
                    }
                    Err(TryRecvError::Closed) => {
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }));
        }
        ThreadPool {
            handles: Some(handles),
            sender,
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.sender.close();
        self.handles
            .take()
            .unwrap()
            .into_iter()
            .for_each(|h| h.join().unwrap());
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
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let signal = Arc::new(Signal::new());
        let waker = Waker::from(signal.clone());
        let mut cx = Context::from_waker(&waker);
        EX.set(self, || {
            let mut fut = std::pin::pin!(future);
            loop {
                if let Poll::Ready(output) = fut.as_mut().poll(&mut cx) {
                    return output;
                }
                while let Some(task) = self.runnable.lock().unwrap().pop_front() {
                    let _ = self.pool.sender.try_send(task);
                }
                signal.wait();
            }
        })
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

pub fn spawn<F: Future<Output = ()> + 'static + Send>(future: F) {
    let signal = Arc::new(Signal::new());
    let task = Arc::new(Task {
        future: RefCell::new(Box::pin(future)),
        signal: signal.clone(),
    });
    EX.with(|ex| {
        ex.runnable.lock().unwrap().push_back(task);
    });
}
