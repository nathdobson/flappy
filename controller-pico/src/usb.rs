use crate::error::Result;
use crate::runtime::reboot;
use core::cell::{RefCell, UnsafeCell};
use core::future::{poll_fn, Future};
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::{bind_interrupts, Peri};
use embassy_sync::blocking_mutex::raw::{
    CriticalSectionRawMutex, NoopRawMutex, ThreadModeRawMutex,
};
use embassy_sync::blocking_mutex::Mutex;
use embassy_usb_logger::ReceiverHandler;
use log::error;
use log::Level;
use static_cell::StaticCell;

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

pub struct UsbModuleBuilder {
    pub spawner: Spawner,
    pub usb: Peri<'static, USB>,
}

pub struct UsbModule {}

impl UsbModuleBuilder {
    pub fn build(self) -> Result<&'static UsbModule> {
        let driver = Driver::new(self.usb, UsbIrqs);
        let ref mut logger_future_ref = *LOGGER_FUTURE.0.borrow_mut();
        if logger_future_ref.is_none() {
            *logger_future_ref = Some(logger_future(driver));
        }
        self.spawner.spawn(logger_task()?);

        static MODULE: StaticCell<UsbModule> = StaticCell::new();
        Ok(MODULE.init(UsbModule {}))
    }
}

type LoggerFuture = impl Future<Output = !>;

#[define_opaque(LoggerFuture)]
fn logger_future(driver: Driver<'static, USB>) -> LoggerFuture {
    async move {
        unsafe {
            static LOGGER: StaticCell<::embassy_usb_logger::UsbLogger<1024, UsbInputHandler>> =
                StaticCell::new();
            let logger = LOGGER.init(::embassy_usb_logger::UsbLogger::with_custom_style(
                |record, writer| {
                    use core::fmt::Write;
                    let level = record.level();
                    let level: &'static str = match level {
                        Level::Error => "\x1B[31merror\x1B[0m",
                        Level::Warn => "\x1B[33mwarn \x1B[0m",
                        Level::Info => "info ",
                        Level::Debug => "debug",
                        Level::Trace => "trace",
                    };
                    let file = record.file().unwrap_or("");
                    let line = record.line().unwrap_or(0);
                    write!(
                        writer,
                        "[{file:20}:{line:5}] [{level}] {}\r\n",
                        record.args()
                    )
                    .ok();
                },
            ));
            logger.with_handler(UsbInputHandler::new());
            let _ = ::log::set_logger_racy(logger)
                .map(|()| log::set_max_level_racy(log::LevelFilter::Info));
            logger
                .run(&mut ::embassy_usb_logger::LoggerState::new(), driver)
                .await
        }
        // embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver, UsbInputHandler);
    }
}

struct LoggerFutureWrapper(RefCell<Option<LoggerFuture>>);

unsafe impl Send for LoggerFutureWrapper {}
unsafe impl Sync for LoggerFutureWrapper {}

static LOGGER_FUTURE: LoggerFutureWrapper = LoggerFutureWrapper(RefCell::new(None));

#[embassy_executor::task]
async fn logger_task() {
    poll_fn(move |cx| unsafe {
        if let Some(future) = &mut *LOGGER_FUTURE.0.borrow_mut() {
            match Pin::new_unchecked(future).poll(cx) {
                Poll::Ready(x) => match x {},
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(())
        }
    })
    .await
}

// Please forgive me, but I couldn't find a better solution.
pub(crate) fn flush_logger() -> ! {
    unsafe {
        loop {
            if let Some(future) = &mut *LOGGER_FUTURE.0.borrow_mut() {
                match Pin::new_unchecked(future).poll(&mut Context::from_waker(&Waker::noop())) {
                    Poll::Ready(x) => match x {},
                    Poll::Pending => {}
                }
            }
        }
    }
}

struct UsbInputHandler;
impl ReceiverHandler for UsbInputHandler {
    async fn handle_data(&self, data: &[u8]) {
        // Control-C generates this byte when run through screen
        if data == &[3u8] {
            reboot();
            return;
        }
        if let Ok(data) = str::from_utf8(data) {
            let data = data.trim();
            log::info!("[USB] Recieved: {:?}", data);
        }
    }

    fn new() -> Self {
        Self
    }
}
