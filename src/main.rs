mod executor;
mod signal;

use executor::spawn;

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
    let (tx, rx) = async_channel::bounded::<i64>(12);
    println!("hello world");
    for i in 0..10 {
        let tx = tx.clone();
        spawn(async move {
            println!("hello world {}", i);
            tx.send(i).await.unwrap();
            // sleep some time
            for _ in 0..100000000 {
                let _ = 1 + 1;
            }
            println!("finish {}, i'm not killed", i);
        });
    }
    for _ in 0..10 {
        println!("recv {}", rx.recv().await.unwrap());
    }
}

fn main() {
    let ex = executor::Executor::new(4);
    ex.block_on(demo());
}
