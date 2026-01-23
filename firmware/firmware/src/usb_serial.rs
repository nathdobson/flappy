use crate::error::Error;
use crate::runtime::{RuntimeModule, reboot_to_bootsel};
use crate::usb::MAX_PACKET_SIZE;
use core::fmt;
use core::fmt::Arguments;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver};
use embassy_sync::mutex::Mutex;
use embassy_sync::pipe::Pipe;
use embassy_time::{Instant, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, ControlChanged, Receiver, Sender, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, UsbDevice};
use heapless::Vec;
use log::{Level, Log, Metadata, Record, info, set_logger, set_max_level};
use crate::make_static;

const LOG_BUFFER: usize = 4096;
const RECEIVE_BUFFER: usize = 4096;
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

const PICO_STDIO_USB_RESET_MAGIC_BAUD_RATE: u32 = 1200;

pub struct UsbSerialModule {
    send_lock: Mutex<CriticalSectionRawMutex, ()>,
    send_buffer: Pipe<CriticalSectionRawMutex, LOG_BUFFER>,
    receive_buffer: Pipe<CriticalSectionRawMutex, RECEIVE_BUFFER>,
    command_buffer: Channel<CriticalSectionRawMutex, Vec<u8, MAX_COMMAND_LEN>, MAX_COMMAND_QUEUE>,
}

impl UsbSerialModule {
    pub fn new() -> &'static Self {
        let module = make_static!(UsbSerialModule, UsbSerialModule {
            send_lock: Mutex::new(()),
            send_buffer: Pipe::new(),
            receive_buffer: Pipe::new(),
            command_buffer: Channel::new(),
        });
        set_logger(module)
            .map(|()| set_max_level(log::LevelFilter::Info))
            .ok();
        module
    }
    pub async fn start(
        &'static self,
        spawner: Spawner,
        builder: &mut Builder<'static, Driver<'static, USB>>,
    ) -> Result<(), Error> {
        let state = make_static!(State, State::new());
        let class = CdcAcmClass::new(builder, state, MAX_PACKET_SIZE as u16);

        let (mut sender, mut receiver, control) = class.split_with_control();

        spawner.spawn({
            #[embassy_executor::task]
            async fn control_task(
                module: &'static UsbSerialModule,
                sender: ControlChanged<'static>,
            ) {
                module.control(sender).await;
            }
            control_task(self, control)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn parse_task(module: &'static UsbSerialModule) {
                module.parse().await;
            }
            parse_task(self)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn terminal_task(
                module: &'static UsbSerialModule,
                mut sender: Sender<'static, Driver<'static, USB>>,
                mut receiver: Receiver<'static, Driver<'static, USB>>,
            ) {
                module.terminal(sender, receiver).await;
            }
            terminal_task(self, sender, receiver)?
        });

        Ok(())
    }
    async fn terminal(
        &self,
        mut sender: Sender<'static, Driver<'static, USB>>,
        mut receiver: Receiver<'static, Driver<'static, USB>>,
    ) {
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
                reboot_to_bootsel();
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
            if control.line_coding().data_rate() == PICO_STDIO_USB_RESET_MAGIC_BAUD_RATE {
                reboot_to_bootsel()
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

impl Log for UsbSerialModule {
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

impl<'a> fmt::Write for &'a UsbSerialModule {
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
