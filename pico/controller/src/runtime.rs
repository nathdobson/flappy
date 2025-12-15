use crate::error::Error;
use crate::product::{PRODUCT_MANUFACTURER, PRODUCT_NAME, serial_number};
use core::fmt;
use core::fmt::Arguments;
use core::intrinsics::abort;
use embassy_executor::{SendSpawner, Spawner};
use embassy_futures::join::join;
use embassy_futures::select::{Either, select};
use embassy_rp::otp::get_chipid;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::{Peri, bind_interrupts, rom_data};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver};
use embassy_sync::mutex::Mutex;
use embassy_sync::pipe::Pipe;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer, block_for};
use embassy_usb::class::cdc_acm::{CdcAcmClass, ControlChanged, Receiver, Sender, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config, UsbDevice};
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
const ESC_CURSOR_SPLIT: &'static str = "\x1B[11;1H";
const ESC_CURSOR_LOG: &'static str = "\x1B[9999;1H";
const ESC_REGION_FEEDBACK: &'static str = "\x1B[3;10r";
const ESC_REGION_LOG: &'static str = "\x1B[12;r";
const ESC_ERASE_ALL: &'static str = "\x1B[2J";
const ESC_ERASE_LINE: &'static str = "\x1B[K";
const ESC_SCROLL: &'static str = "\x1B[1S";
const ESC_BANNER1: &'static str = "\x1B[61;11;1;11;500$x";
const ESC_BANNER2: &'static str = "\x1B[61;2;1;2;500$x";
const ESC_INVERT: &'static str = "\x1B[7m";
const ESC_NORMAL: &'static str = "\x1B[0m";

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});
#[allow(non_snake_case)]
pub struct RuntimePeripherals {
    pub USB: Peri<'static, USB>,
}

pub struct RuntimeModule {
    send_lock: Mutex<CriticalSectionRawMutex, ()>,
    send_buffer: Pipe<CriticalSectionRawMutex, LOG_BUFFER>,
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
            send_lock: Mutex::new(()),
            send_buffer: Pipe::new(),
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

        let (mut sender, mut receiver, control) = class.split_with_control();
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
            async fn parse_task(module: &'static RuntimeModule) {
                module.parse().await;
            }
            parse_task(self)?
        });
        loop {
            sender.wait_connection().await;
            receiver.wait_connection().await;
            match select(
                async {
                    loop {
                        sender.write_packet(b"\x1B[5n").await?;
                        Timer::after_millis(1000).await;
                    }
                    Ok::<_, EndpointError>(())
                },
                async {
                    loop {
                        let mut packet = [0u8; 10];
                        let len = receiver.read_packet(&mut packet).await?;
                        if packet[..len] == *b"\x1B[0n" {
                            break;
                        }
                    }
                    Ok::<_, EndpointError>(())
                },
            )
            .await
            {
                Either::First(Err(_)) | Either::Second(Err(_)) => continue,
                _ => {}
            };
            select(
                async {
                    let mut buf = [0; MAX_PACKET_SIZE as usize];
                    let mut inited = false;
                    while Err(EndpointError::Disabled)
                        != try {
                            if !inited {
                                inited = true;
                                sender.write_packet(ESC_ERASE_ALL.as_bytes()).await?;
                                sender.write_packet(ESC_CURSOR_INPUT.as_bytes()).await?;
                                sender.write_packet(">".as_bytes()).await?;
                                sender.write_packet(ESC_INVERT.as_bytes()).await?;
                                sender.write_packet(ESC_BANNER1.as_bytes()).await?;
                                sender.write_packet(ESC_BANNER2.as_bytes()).await?;
                                sender.write_packet(ESC_NORMAL.as_bytes()).await?;
                            }
                            let len = self.send_buffer.read(&mut buf[..]).await;
                            sender.write_packet(&buf[..len]).await?;
                            if len == MAX_PACKET_SIZE as usize {
                                sender.write_packet(&[]).await?;
                            }
                        }
                    {}
                },
                async {
                    let mut buf = [0u8; MAX_PACKET_SIZE as usize];
                    while Err(EndpointError::Disabled)
                        != try {
                            let len = receiver.read_packet(&mut buf[..]).await?;
                            self.receive_buffer.write_all(&buf[..len]).await;
                        }
                    {}
                },
            )
            .await;
        }
        Ok(())
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
                {
                    let lock = self.send_lock.lock().await;
                    self.send_buffer.write(ESC_CURSOR_INPUT.as_bytes()).await;
                    self.send_buffer.write(ESC_ERASE_LINE.as_bytes()).await;
                    self.send_buffer.write(b">").await;
                }
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
            } else if b == 127 {
                if let Some(n) = command.pop() {
                    self.send_buffer.write(b"\x08\x1B[P").await;
                }
            } else if b.is_ascii_control() {
                info!("Ascii {}", b);
            } else {
                if let Ok(_) = command.push(b) {
                    let lock = self.send_lock.lock().await;
                    self.send_buffer.write_all(&[b]).await;
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
        let time = Instant::now().as_secs();
        write!(
            writer,
            "{ESC_SAVE}{ESC_REGION_LOG}{ESC_CURSOR_LOG}{ESC_SCROLL}[{file:30}:{line:5}] [{time:8} S] [{level}] {}{ESC_RESTORE}",
            record.args()
        )
    }
    pub fn commands(&'static self) -> DynamicReceiver<'static, Vec<u8, MAX_COMMAND_LEN>> {
        self.command_buffer.dyn_receiver()
    }
    pub async fn write_feedback_line(mut self: &'static Self, args: Arguments<'_>) {
        use core::fmt::Write;
        let lock = self.send_lock.lock().await;
        write!(
            &mut self,
            "{ESC_SAVE}{ESC_REGION_FEEDBACK}{ESC_CURSOR_FEEDBACK}{ESC_SCROLL}{args}{ESC_RESTORE}"
        )
        .ok();
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
            if let Ok(guard) = self.send_lock.try_lock() {
                Self::write_record(record, &mut self).ok();
            }
        }
    }

    fn flush(&self) {}
}

impl<'a> core::fmt::Write for &'a RuntimeModule {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let b = s.as_bytes();
        let mut len = self.send_buffer.try_write(b).map_err(|_| fmt::Error)?;
        if len < s.len() {
            len += self
                .send_buffer
                .try_write(&b[len..])
                .map_err(|_| fmt::Error)?;
            if len < s.len() {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}
