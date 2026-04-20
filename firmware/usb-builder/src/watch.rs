use core::cell::RefCell;
use core::future::poll_fn;
use core::task::Poll;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::waitqueue::GenericAtomicWaker;

pub struct Watch<M: RawMutex, T> {
    state: Mutex<M, RefCell<T>>,
    waker: GenericAtomicWaker<M>,
}

impl<M: RawMutex, T> Watch<M, T> {
    pub fn new(value: T) -> Self {
        Watch {
            state: Mutex::new(RefCell::new(value)),
            waker: GenericAtomicWaker::new(M::INIT),
        }
    }
    pub fn modify<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        self.state.lock(|x| {
            let result = f(&mut *x.borrow_mut());
            self.waker.wake();
            result
        })
    }
    pub async fn wait_until(&self, mut f: impl FnMut(&T) -> bool) {
        poll_fn(|cx| {
            if self.state.lock(|x| f(&*x.borrow())) {
                return Poll::Ready(());
            } else {
                self.waker.register(cx.waker());
                if self.state.lock(|x| f(&*x.borrow())) {
                    return Poll::Ready(());
                }
            }
            Poll::Pending
        })
        .await;
    }
    pub fn read<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.state.lock(|x| f(&*x.borrow()))
    }
}
