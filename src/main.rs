mod executor;
mod signal;

use async_std::task::spawn;

// use std::{
//     future::Future,
//     task::{Context, RawWaker, RawWakerVTable, Waker},
// };
// struct Demo;

// impl Future for Demo {
//     type Output = ();
//     fn poll(
//         self: std::pin::Pin<&mut Self>,
//         _cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Self::Output> {
//         println!("hello world");
//         std::task::Poll::Ready(())
//     }
// }

// fn dummy_waker() -> Waker {
//     static DATA: () = ();
//     unsafe { Waker::from_raw(RawWaker::new(&DATA, &VTABLE)) }
// }

// const VTABLE: RawWakerVTable =
//     RawWakerVTable::new(vtable_clone, vtable_wake, vtable_wake_by_ref, vtable_drop);

// unsafe fn vtable_clone(_p: *const ()) -> RawWaker {
//     RawWaker::new(_p, &VTABLE)
// }

// unsafe fn vtable_wake(_p: *const ()) {}
// unsafe fn vtable_wake_by_ref(_p: *const ()) {}
// unsafe fn vtable_drop(_p: *const ()) {}

// fn block_on<F: Future>(future: F) -> F::Output {
//     let mut fut = std::pin::pin!(future);
//     let waker = dummy_waker();
//     let mut cx = Context::from_waker(&waker);
//     loop {
//         if let std::task::Poll::Ready(output) = fut.as_mut().poll(&mut cx) {
//             return output;
//         }
//     }
// }

async fn demo() {
    let (tx, rx) = async_channel::bounded(1);
    spawn(demo2(tx));
    println!("hello world");
    let _ = rx.recv().await;
}

async fn demo2(tx: async_channel::Sender<()>) {
    println!("hello world2");
    let _ = tx.send(()).await;
}
fn main() {
    let ex = executor::Executor::new(1);
    ex.block_on(demo());
}
