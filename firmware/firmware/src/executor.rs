use core::future::Future;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_executor::{Executor, InterruptExecutor, SendSpawner, Spawner};
use embassy_rp::interrupt::{InterruptExt, Priority};
use embassy_rp::peripherals::UART0;
use embassy_rp::uart::InterruptHandler;
use embassy_rp::{bind_interrupts, interrupt};
use make_static::make_static;

#[cfg(feature = "preemption")]
static RUNTIME_EXECUTOR: InterruptExecutor = InterruptExecutor::new();
#[cfg(feature = "preemption")]
static RUNTIME_STARTED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "preemption")]
#[interrupt]
unsafe fn SWI_IRQ_0() {
    if RUNTIME_STARTED.load(Ordering::SeqCst) {
        unsafe { RUNTIME_EXECUTOR.on_interrupt() }
    }
}

#[cfg(feature = "preemption")]
pub fn run_program<T>(runtime: impl FnOnce(SendSpawner) -> T, app: impl FnOnce(Spawner, T)) -> ! {
    interrupt::SWI_IRQ_0.set_priority(Priority::P3);
    let runtime_spawner = RUNTIME_EXECUTOR.start(interrupt::SWI_IRQ_0);
    RUNTIME_STARTED.store(true, Ordering::SeqCst);
    let data = runtime(runtime_spawner);
    let application_executor = make_static!(Executor, Executor::new());
    application_executor.run(|s| app(s, data))
}

#[cfg(not(feature = "preemption"))]
pub fn run_program<T>(runtime: impl FnOnce(SendSpawner) -> T, app: impl FnOnce(Spawner, T)) -> ! {
    let application_executor = make_static!(Executor, Executor::new());
    application_executor.run(|s| app(s, runtime(s.make_send())));
}
