use crate::error::Error;
use crate::product::{PRODUCT_MANUFACTURER, PRODUCT_NAME, serial_number};
use core::fmt;
use core::intrinsics::abort;
use embassy_executor::{SendSpawner, Spawner};
use embassy_futures::join::join;
use embassy_rp::otp::get_chipid;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::{Peri, bind_interrupts, rom_data};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver};
use embassy_sync::pipe::Pipe;
use embassy_time::{Duration, Instant, block_for};
use embassy_usb::class::cdc_acm::{CdcAcmClass, ControlChanged, Receiver, Sender, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config, UsbDevice};
use embedded_io_async::{Read, Write};
use heapless::{String, Vec};
use log::{Level, Log, Metadata, Record, error, info, set_logger, set_max_level};
use static_cell::make_static;

const MODULE: &'static str = "[RUN  ]";
const LOG_BUFFER: usize = 4096;
const RECEIVE_BUFFER: usize = 4096;
const MAX_PACKET_SIZE: u8 = 64;
const MAX_COMMAND_LEN: usize = 128;
const MAX_COMMAND_QUEUE: usize = 1;
const MAX_ESCAPE: usize = 10;

const ESC_SAVE: &'static str = "\x1B[s";
const ESC_RESTORE: &'static str = "\x1B[u";
const ESC_CURSOR_INPUT: &'static str = "\x1B[1;1H";
const ESC_CURSOR_FEEDBACK: &'static str = "\x1B[10;1H";
const ESC_CURSOR_LOG: &'static str = "\x1B[9999;1H";
const ESC_REGION_FEEDBACK: &'static str = "\x1B[2;10r";
const ESC_REGION_LOG: &'static str = "\x1B[11;r";
const ESC_ERASE_ALL: &'static str = "\x1B[2J";
const ESC_ERASE_LINE: &'static str = "\x1B[K";
const SCROLL:&'static str= "\x1B[1S";

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});
#[allow(non_snake_case)]
pub struct RuntimePeripherals {
    pub USB: Peri<'static, USB>,
}

pub struct RuntimeModule {
    log_buffer: Pipe<CriticalSectionRawMutex, LOG_BUFFER>,
    receive_buffer: Pipe<CriticalSectionRawMutex, RECEIVE_BUFFER>,
    command_buffer: Channel<CriticalSectionRawMutex, Vec<u8, MAX_COMMAND_LEN>, MAX_COMMAND_QUEUE>,
}

struct RuntimeState {
    state: State<'static>,
    config_descriptor: [u8; 128],
    bos_descriptor: [u8; 16],
    msos_descriptor: [u8; 256],
    control_buf: [u8; 64],
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}

impl RuntimeModule {
    pub fn new(spawner: SendSpawner, peri: RuntimePeripherals) -> &'static Self {
        let module: &'static RuntimeModule = make_static!(RuntimeModule {
            log_buffer: Pipe::new(),
            receive_buffer: Pipe::new(),
            command_buffer: Channel::new(),
        });
        set_logger(module)
            .map(|()| set_max_level(log::LevelFilter::Info))
            .ok();
        spawner.spawn({
            #[embassy_executor::task]
            async fn start_task(module: &'static RuntimeModule, peri: RuntimePeripherals) {
                module.start(peri).await.unwrap();
            }
            start_task(module, peri).unwrap()
        });
        module
    }
    async fn start(&'static self, peri: RuntimePeripherals) -> Result<(), Error> {
        let spawner = unsafe { Spawner::for_current_executor().await };
        let driver = Driver::new(peri.USB, UsbIrqs);

        let mut config = Config::new(0x2e8a, 0x000f);
        config.manufacturer = Some(PRODUCT_MANUFACTURER);
        config.product = Some(PRODUCT_NAME);

        config.serial_number = serial_number();
        config.max_power = 100;
        config.max_packet_size_0 = MAX_PACKET_SIZE;
        let state = make_static!(RuntimeState {
            state: State::new(),
            config_descriptor: [0; 128],
            bos_descriptor: [0; 16],
            msos_descriptor: [0; 256],
            control_buf: [0; 64],
        });
        let mut builder = Builder::new(
            driver,
            config,
            &mut state.config_descriptor,
            &mut state.bos_descriptor,
            &mut state.msos_descriptor,
            &mut state.control_buf,
        );
        let class = CdcAcmClass::new(&mut builder, &mut state.state, MAX_PACKET_SIZE as u16);

        let mut device = builder.build();

        let (sender, receiver, control) = class.split_with_control();
        spawner.spawn({
            #[embassy_executor::task]
            async fn device_task(mut device: UsbDevice<'static, Driver<'static, USB>>) -> ! {
                device.run().await
            }
            device_task(device)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn control_task(module: &'static RuntimeModule, sender: ControlChanged<'static>) {
                module.control(sender).await;
            }
            control_task(self, control)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn send_task(
                module: &'static RuntimeModule,
                sender: Sender<'static, Driver<'static, USB>>,
            ) {
                module.send(sender).await;
            }
            send_task(self, sender)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn receive_task(
                module: &'static RuntimeModule,
                receiver: Receiver<'static, Driver<'static, USB>>,
            ) {
                module.receive(receiver).await;
            }
            receive_task(self, receiver)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn parse_task(module: &'static RuntimeModule) {
                module.parse().await;
            }
            parse_task(self)?
        });
        Ok(())
    }
    async fn send(&'static self, mut sender: Sender<'static, Driver<'static, USB>>) {
        let mut buf = [0; MAX_PACKET_SIZE as usize];
        loop {
            sender.wait_connection().await;
            sender.write_packet(ESC_ERASE_ALL.as_bytes()).await.ok();
            sender.write_packet(ESC_CURSOR_INPUT.as_bytes()).await.ok();
            while Err(EndpointError::Disabled)
                != try {
                    let len = self.log_buffer.read(&mut buf[..]).await;
                    sender.write_packet(&buf[..len]).await?;
                    if len == MAX_PACKET_SIZE as usize {
                        sender.write_packet(&[]).await?;
                    }
                }
            {}
        }
    }
    async fn receive(&'static self, mut receiver: Receiver<'static, Driver<'static, USB>>) {
        let mut command = Vec::<u8, MAX_COMMAND_LEN>::new();
        let mut buf = [0u8; MAX_PACKET_SIZE as usize];
        loop {
            receiver.wait_connection().await;
            while Err(EndpointError::Disabled)
                != try {
                    let len = receiver.read_packet(&mut buf[..]).await?;
                    self.receive_buffer.write_all(&buf[..len]).await;
                }
            {}
        }
    }
    async fn read_u8(&'static self) -> u8 {
        loop {
            let mut b = [0u8];
            if self.receive_buffer.read(&mut b).await < 1 {
                continue;
            }
            return b[0];
        }
    }
    async fn parse(&'static self) {
        let mut command = Vec::<u8, MAX_COMMAND_LEN>::new();
        loop {
            let b = self.read_u8().await;
            if b == 3 {
                reboot();
            } else if b == b'\r' {
                self.command_buffer.send(command.clone()).await;
                command.clear();
                self.log_buffer.write(b"\r").await;
                self.log_buffer.write(ESC_ERASE_LINE.as_bytes()).await;
            } else if b == b'\x1B' {
                let mut escape = Vec::<u8, MAX_ESCAPE>::new();
                loop {
                    let b = self.read_u8().await;
                    escape.push(b).ok();
                    if b.is_ascii_alphabetic() {
                        break;
                    }
                }
                info!("Escape {:?}", escape);
            } else {
                if let Ok(_) = command.push(b) {
                    self.log_buffer.write_all(&[b]).await;
                }
            }
        }
    }
    async fn control(&'static self, control: ControlChanged<'static>) {
        loop {
            control.control_changed().await;
            // Allow out-of-band reset of the device
            if control.line_coding().data_rate() == 50 {
                reboot();
            }
        }
    }
    fn write_record(record: &Record, writer: &mut impl core::fmt::Write) -> core::fmt::Result {
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
        let time = Instant::now().as_millis() as f64 / 1000.0;
        write!(
            writer,
            "{ESC_SAVE}{ESC_REGION_LOG}{ESC_CURSOR_LOG}{SCROLL}[{file:20}:{line:5}] [{time:7.3} S] [{level}] {}{ESC_RESTORE}",
            record.args()
        )
    }
    pub fn commands(&'static self) -> DynamicReceiver<'static, Vec<u8, MAX_COMMAND_LEN>> {
        self.command_buffer.dyn_receiver()
    }
}

pub fn reboot() {
    rom_data::reboot(0x0002, 500, 0, 0);
}

impl Log for RuntimeModule {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(mut self: &Self, record: &Record) {
        if self.enabled(record.metadata()) {
            Self::write_record(record, &mut self).ok();
        }
    }

    fn flush(&self) {}
}

impl<'a> core::fmt::Write for &'a RuntimeModule {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let b = s.as_bytes();
        let mut len = self.log_buffer.try_write(b).map_err(|_| fmt::Error)?;
        if len < s.len() {
            len += self
                .log_buffer
                .try_write(&b[len..])
                .map_err(|_| fmt::Error)?;
            if len < s.len() {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}
