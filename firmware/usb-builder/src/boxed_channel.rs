use alloc::boxed::Box;
use core::ops::Deref;
use core::ops::DerefMut;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::channel;
use embassy_sync::semaphore::{GreedySemaphore, Semaphore, SemaphoreReleaser};
use fixed_freelist::{Freelist, FreelistStorage};

pub struct BoxedChannel<M: 'static + RawMutex, T, const N: usize> {
    channel: channel::Channel<M, ReceiveGuard<M, T>, N>,
    freelist: FreelistStorage<M, T, N>,
    semaphore: GreedySemaphore<M>,
}

pub struct ReceiveGuard<M: 'static + RawMutex, T> {
    value: Box<T, Freelist<'static, M>>,
    guard: SemaphoreReleaser<'static, GreedySemaphore<M>>,
}

pub struct SendBuilder<M: 'static + RawMutex, T: 'static> {
    sender: channel::DynamicSender<'static, ReceiveGuard<M, T>>,
    message: ReceiveGuard<M, T>,
}

impl<M: 'static + RawMutex, T, const N: usize> BoxedChannel<M, T, N> {
    pub fn new() -> Self {
        BoxedChannel {
            channel: channel::Channel::new(),
            freelist: FreelistStorage::new(),
            semaphore: GreedySemaphore::new(N),
        }
    }
    #[must_use]
    pub async fn alloc<F: FnOnce() -> T>(&'static self, value: F) -> SendBuilder<M, T> {
        let guard = match self.semaphore.acquire(1).await {
            Ok(guard) => guard,
            Err(e) => match e {},
        };
        let value = self.freelist.alloc_box_with(value).unwrap();
        SendBuilder {
            sender: self.channel.dyn_sender(),
            message: ReceiveGuard { value, guard },
        }
    }

    pub async fn receive(&'static self) -> ReceiveGuard<M, T> {
        self.channel.receive().await
    }
}

impl<M: 'static + RawMutex, T> Deref for ReceiveGuard<M, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.value
    }
}

impl<M: 'static + RawMutex, T> DerefMut for ReceiveGuard<M, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.value
    }
}

impl<M: 'static + RawMutex, T> Deref for SendBuilder<M, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.message.value
    }
}

impl<M: 'static + RawMutex, T> DerefMut for SendBuilder<M, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.message.value
    }
}

impl<M: 'static + RawMutex, T> SendBuilder<M, T> {
    pub fn send(self) {
        self.sender.try_send(self.message).ok().unwrap();
    }
}
