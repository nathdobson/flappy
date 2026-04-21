#![no_std]
#![feature(type_alias_impl_trait)]
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_executor::raw::TaskPool;
use embassy_executor::{Executor, Spawner};
use embassy_executor::{InterruptExecutor, SendSpawner};
use embassy_rp::interrupt;
use embassy_rp::interrupt::{InterruptExt, Priority};
use make_static::make_static;

static RUNTIME_EXECUTOR: InterruptExecutor = InterruptExecutor::new();
static RUNTIME_STARTED: AtomicBool = AtomicBool::new(false);

#[interrupt]
unsafe fn SWI_IRQ_0() {
    if RUNTIME_STARTED.load(Ordering::SeqCst) {
        unsafe { RUNTIME_EXECUTOR.on_interrupt() }
    }
}

pub struct Runtime {
    pub interrupt: SendSpawner,
    pub thread: Spawner,
}

pub fn start_runtime(runner: impl FnOnce(Runtime)) -> ! {
    interrupt::SWI_IRQ_0.set_priority(Priority::P3);
    let interrupt = RUNTIME_EXECUTOR.start(interrupt::SWI_IRQ_0);
    RUNTIME_STARTED.store(true, Ordering::SeqCst);
    let thread = make_static!(Executor, Executor::new());
    thread.run(|thread| runner(Runtime { interrupt, thread }))
}

impl<F: 'static + Send + FnOnce(Spawner) -> Fu, Fu: 'static + Future<Output = ()>>
    RemoteSpawner<F, Fu>
{
    pub fn new() -> Self {
        RemoteSpawner {
            runner_pool: TaskPool::new(),
            future_pool: TaskPool::new(),
        }
    }

    pub fn spawn(&'static self, spawner: SendSpawner, inner: F) {
        spawner.spawn(self.runner_pool.spawn(|| runner(self, inner)).unwrap());
    }
}

type Runner<F: 'static + Send + FnOnce(Spawner) -> Fu, Fu: 'static + Future<Output = ()>> =
    impl Send + Future<Output = ()>;

#[define_opaque(Runner)]
fn runner<F: 'static + Send + FnOnce(Spawner) -> Fu, Fu: 'static + Future<Output = ()>>(
    this: &'static RemoteSpawner<F, Fu>,
    inner: F,
) -> Runner<F, Fu> {
    async move {
        let spawner = unsafe { Spawner::for_current_executor().await };
        spawner.spawn(this.future_pool.spawn(|| inner(spawner)).unwrap())
    }
}

pub struct RemoteSpawner<
    F: 'static + Send + FnOnce(Spawner) -> Fu,
    Fu: 'static + Future<Output = ()>,
> {
    runner_pool: TaskPool<Runner<F, Fu>, 1>,
    future_pool: TaskPool<Fu, 1>,
}
