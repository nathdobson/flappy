use core::future::Future;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_executor::{Executor, InterruptExecutor, SendSpawner, Spawner};
use embassy_rp::interrupt::{InterruptExt, Priority};
use embassy_rp::peripherals::UART0;
use embassy_rp::uart::InterruptHandler;
use embassy_rp::{bind_interrupts, interrupt};
use static_cell::make_static;

static RUNTIME_EXECUTOR: InterruptExecutor = InterruptExecutor::new();
static RUNTIME_STARTED: AtomicBool = AtomicBool::new(false);

#[interrupt]
unsafe fn SWI_IRQ_0() {
    if RUNTIME_STARTED.load(Ordering::SeqCst) {
        unsafe { RUNTIME_EXECUTOR.on_interrupt() }
    }
}

pub fn run_program(runtime: impl FnOnce(SendSpawner), app: impl FnOnce(Spawner)) -> ! {
    interrupt::SWI_IRQ_0.set_priority(Priority::P3);
    let runtime_spawner = RUNTIME_EXECUTOR.start(interrupt::SWI_IRQ_0);
    RUNTIME_STARTED.store(true, Ordering::SeqCst);
    runtime(runtime_spawner);
    let application_executor = make_static!(Executor::new());
    application_executor.run(app)
}
