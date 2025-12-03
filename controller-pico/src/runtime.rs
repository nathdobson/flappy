use crate::product::{PRODUCT_MANUFACTURER, PRODUCT_NAME};
use core::fmt;
use core::intrinsics::abort;
use embassy_executor::{SendSpawner, Spawner};
use embassy_futures::join::join;
use embassy_rp::otp::get_chipid;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::{Peri, bind_interrupts, rom_data};
use embassy_time::{Duration, block_for};
use embassy_usb::class::cdc_acm::{CdcAcmClass, ControlChanged, State};
use embassy_usb::{Builder, Config, UsbDevice};
// use embassy_usb_dfu::consts::DfuAttributes;
// use embassy_usb_dfu::{Control, DfuMarker, Reset, usb_dfu};
use embassy_usb_logger::{LoggerState, MAX_PACKET_SIZE, ReceiverHandler, UsbLogger};
use heapless::String;
use log::{Level, Record, error, info, set_logger, set_max_level};
use static_cell::StaticCell;

const LOG_BUFFER: usize = 1024;

#[allow(non_snake_case)]
pub struct RuntimePeripherals {
    pub USB: Peri<'static, USB>,
}

type MyLogger = UsbLogger<LOG_BUFFER, UsbInputHandler>;

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

pub fn reboot() {
    rom_data::reboot(0x0002, 500, 0, 0);
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}

struct UsbInputHandler;
impl ReceiverHandler for UsbInputHandler {
    async fn handle_data(&self, data: &[u8]) {
        // Control-C generates this byte when run through screen
        if data == &[3u8] {
            reboot();
            return;
        }
        info!("Received data: {:?}", data);
        if let Ok(data) = str::from_utf8(data) {
            let data = data.trim();
        }
    }

    fn new() -> Self {
        Self
    }
}

fn custom_style(record: &Record, writer: &mut embassy_usb_logger::Writer<LOG_BUFFER>) {
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
}

#[embassy_executor::task]
async fn control_changed(control: ControlChanged<'static>) {
    loop {
        control.control_changed().await;
        // All out-of-band reset of the device
        if control.line_coding().data_rate() == 50 {
            reboot();
        }
    }
}

#[embassy_executor::task]
async fn log_task(
    spawner: Spawner,
    logger: &'static MyLogger,
    class: CdcAcmClass<'static, Driver<'static, USB>>,
) -> ! {
    let (mut sender, mut receiver, control) = class.split_with_control();
    spawner.spawn(control_changed(control).unwrap());
    loop {
        logger.run_logger_class(&mut sender, &mut receiver).await;
    }
}

#[embassy_executor::task]
async fn run_device(mut device: UsbDevice<'static, Driver<'static, USB>>) -> ! {
    device.run().await
}

struct RuntimeState {
    state: State<'static>,
    config_descriptor: [u8; 128],
    bos_descriptor: [u8; 16],
    msos_descriptor: [u8; 256],
    control_buf: [u8; 64],
}

#[embassy_executor::task]
async fn start_runtime(rp: RuntimePeripherals, logger: &'static MyLogger) {
    let spawner = unsafe { Spawner::for_current_executor().await };
    let driver = Driver::new(rp.USB, UsbIrqs);

    let mut config = Config::new(0x2e8a, 0x000f);
    config.manufacturer = Some(PRODUCT_MANUFACTURER);
    config.product = Some(PRODUCT_NAME);

    if let Ok(serial) = get_chipid() {
        use core::fmt::Write;
        static SERIAL_NUMBER: StaticCell<String<128>> = StaticCell::new();
        let serial_number = SERIAL_NUMBER.init(String::new());
        write!(serial_number, "{:016X}", serial).ok();
        config.serial_number = Some(serial_number);
    }
    config.max_power = 100;
    config.max_packet_size_0 = MAX_PACKET_SIZE;
    static RUNTIME_STATE: StaticCell<RuntimeState> = StaticCell::new();
    let runtime_state = RUNTIME_STATE.init(RuntimeState {
        state: State::new(),
        config_descriptor: [0; 128],
        bos_descriptor: [0; 16],
        msos_descriptor: [0; 256],
        control_buf: [0; 64],
    });
    let mut builder = Builder::new(
        driver,
        config,
        &mut runtime_state.config_descriptor,
        &mut runtime_state.bos_descriptor,
        &mut runtime_state.msos_descriptor,
        &mut runtime_state.control_buf,
    );
    let class = CdcAcmClass::new(
        &mut builder,
        &mut runtime_state.state,
        MAX_PACKET_SIZE as u16,
    );

    let mut device = builder.build();

    spawner.spawn(run_device(device).unwrap());
    spawner.spawn(log_task(spawner, logger, class).unwrap());
}

pub fn runtime(spawner: SendSpawner, rp: RuntimePeripherals) {
    static LOGGER: StaticCell<MyLogger> = StaticCell::new();
    let logger = LOGGER.init(UsbLogger::with_custom_style(custom_style));
    logger.with_handler(UsbInputHandler::new());
    let _ = set_logger(logger).map(|()| set_max_level(log::LevelFilter::Info));
    spawner.spawn(start_runtime(rp, logger).unwrap());
}
