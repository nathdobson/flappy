use crate::error::Error;
use crate::watch::Watch;
use crate::{MAX_PACKET_SIZE, UsbBuilder, UsbServer};
use core::fmt;
use core::fmt::Arguments;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::channel::{Channel, DynamicReceiver};
use embassy_sync::mutex::Mutex;
use embassy_sync::pipe::Pipe;
use embassy_time::{Delay, Instant, Timer};
use embassy_usb::Builder;
use embassy_usb::class::cdc_acm::{CdcAcmClass, ControlChanged, Receiver, Sender, State};
use embassy_usb::driver::EndpointError;
use embedded_hal_async::delay::DelayNs;
use embedded_io::{ErrorType, Write, WriteFmtError};
use heapless::{Vec, VecView};
use log::{Level, Log, Metadata, Record, info, set_logger, set_max_level};
use log_vec::{LogVec, SlicePair};
use make_static::make_static;
use reboot::reboot_bootsel_after;

const LOG_BUFFER_BYTES: usize = 4096;
const RENDER_BYTES: usize = 4096;
const LOG_BUFFER_PACKETS: usize = 128;
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
const BACKSPACE: &'static str = "\x08\x1B[P";

const PICO_STDIO_USB_RESET_MAGIC_BAUD_RATE: u32 = 1200;

enum LogLine {
    Feedback,
    Log,
}
type UsbLogVec = LogVec<LogLine, LOG_BUFFER_PACKETS, LOG_BUFFER_BYTES>;
type UsbLogWatch = Watch<CriticalSectionRawMutex, UsbLogVec>;
type UsbSender = Mutex<NoopRawMutex, Sender<'static, Driver<'static, USB>>>;
type UsbReceiver = Receiver<'static, Driver<'static, USB>>;

pub struct UsbTerminalServer {
    log: UsbLogWatch,
    receive_buffer: Pipe<CriticalSectionRawMutex, RECEIVE_BUFFER>,
    command_buffer: Channel<CriticalSectionRawMutex, Vec<u8, MAX_COMMAND_LEN>, MAX_COMMAND_QUEUE>,
}
impl UsbTerminalServer {
    pub fn new() -> Self {
        UsbTerminalServer {
            log: Watch::new(LogVec::new()),
            receive_buffer: Pipe::new(),
            command_buffer: Channel::new(),
        }
    }
    pub fn set_logger(&'static self) {
        set_logger(self)
            .map(|()| set_max_level(log::LevelFilter::Info))
            .ok();
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
    async fn parse(&'static self, sender: &UsbSender) {
        let mut command = Vec::<u8, MAX_COMMAND_LEN>::new();
        loop {
            let b = self.read_u8().await;
            if b == 3 {
                reboot_bootsel_after(10);
                info!("Rebooting...");
            } else if b == b'\r' {
                self.command_buffer.send(command.clone()).await;
                command.clear();
                {
                    let mut sender = sender.lock().await;
                    sender.write_packet(ESC_CURSOR_INPUT.as_bytes()).await.ok();
                    sender.write_packet(ESC_ERASE_LINE.as_bytes()).await.ok();
                    sender.write_packet(b">").await.ok();
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
                    sender
                        .lock()
                        .await
                        .write_packet(BACKSPACE.as_bytes())
                        .await
                        .ok();
                }
            } else if b.is_ascii_control() {
                info!("Ascii {}", b);
            } else {
                if let Ok(_) = command.push(b) {
                    sender.lock().await.write_packet(&[b]).await.ok();
                }
            }
        }
    }
    async fn control(&'static self, control: ControlChanged<'static>) {
        loop {
            control.control_changed().await;
            // Allow out-of-band reset of the device
            if control.line_coding().data_rate() == PICO_STDIO_USB_RESET_MAGIC_BAUD_RATE {
                reboot_bootsel_after(10);
            }
        }
    }
    fn write_record<W: embedded_io::Write>(record: &Record, writer: &mut W) {
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
            "[{file:30}:{line:5}] [{time:8} S] [{level}] {}",
            record.args()
        )
        .ok();
    }
    pub fn commands(&'static self) -> DynamicReceiver<'static, Vec<u8, MAX_COMMAND_LEN>> {
        self.command_buffer.dyn_receiver()
    }
    pub fn write_feedback_line(mut self: &'static Self, args: Arguments<'_>) {
        self.log.modify(|log| {
            let mut line = log.push_back();
            write!(line, "{}", args).ok();
            line.build(LogLine::Feedback);
        })
    }

    async fn wait_for_terminal(
        &self,
        sender: &UsbSender,
        receiver: &mut UsbReceiver,
    ) -> Result<(), EndpointError> {
        let result = select(
            async {
                loop {
                    sender.lock().await.write_packet(b"\x1B[5n").await?;
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
        .await;
        match result {
            Either::First(x) | Either::Second(x) => x?,
        };
        Ok(())
    }
    async fn write_packets(
        &'static self,
        sender: &UsbSender,
        render_buffer: &mut VecView<u8>,
    ) -> Result<!, EndpointError> {
        let mut next_packet = 0;
        {
            let mut sender = sender.lock().await;
            sender.write_packet(ESC_ERASE_ALL.as_bytes()).await?;
            sender.write_packet(ESC_CURSOR_INPUT.as_bytes()).await?;
            sender.write_packet(">".as_bytes()).await?;
            sender.write_packet(ESC_INVERT.as_bytes()).await?;
            sender.write_packet(ESC_BANNER1.as_bytes()).await?;
            sender.write_packet(ESC_BANNER2.as_bytes()).await?;
            sender.write_packet(ESC_NORMAL.as_bytes()).await?;
        }
        loop {
            render_buffer.clear();
            if self
                .log
                .read(|log| {
                    //
                    if next_packet < log.packet_range().start {
                        write!(
                            render_buffer,
                            "missing {} lines",
                            log.packet_range().start - next_packet
                        )
                        .ok()?;
                        next_packet = log.packet_range().start
                    }
                    if let Some((value, data)) = log.packet(next_packet) {
                        next_packet += 1;
                        self.render_line(render_buffer, value, data)?;
                    }
                    Some(())
                })
                .is_none()
            {
                continue;
            }
            {
                let mut sender = sender.lock().await;
                for packet in render_buffer.chunks(MAX_PACKET_SIZE as usize) {
                    sender.write_packet(packet).await?;
                }
                if render_buffer.len() % MAX_PACKET_SIZE as usize == 0 {
                    sender.write_packet(&[]).await?;
                }
            }
            self.log
                .wait_until(|log| log.packet_range().end > next_packet)
                .await;
        }
    }
    fn render_line(
        &self,
        render_buffer: &mut VecView<u8>,
        value: &LogLine,
        data: SlicePair<u8>,
    ) -> Option<()> {
        match value {
            LogLine::Feedback => {
                write!(
                    render_buffer,
                    "{ESC_SAVE}{ESC_REGION_FEEDBACK}{ESC_CURSOR_FEEDBACK}{ESC_SCROLL}"
                )
                .ok()?;
            }
            LogLine::Log => {
                write!(
                    render_buffer,
                    "{ESC_SAVE}{ESC_REGION_LOG}{ESC_CURSOR_LOG}{ESC_SCROLL}"
                )
                .ok()?;
            }
        }
        render_buffer.extend_from_slice(data.slice1()).ok()?;
        render_buffer.extend_from_slice(data.slice2()).ok()?;
        write!(render_buffer, "{ESC_RESTORE}").ok()?;

        Some(())
    }
    async fn read_packets(&'static self, receiver: &mut UsbReceiver) -> Result<!, EndpointError> {
        loop {
            let mut buf = [0u8; MAX_PACKET_SIZE as usize];
            let len = receiver.read_packet(&mut buf[..]).await?;
            self.receive_buffer.write_all(&buf[..len]).await;
        }
    }
    async fn do_packets(&'static self, sender: &'static UsbSender, mut receiver: UsbReceiver) {
        let mut render_buffer = make_static! {Vec<u8, RENDER_BYTES>, Vec::new()};
        loop {
            sender.lock().await.wait_connection().await;
            receiver.wait_connection().await;
            if let Err(_) = self.wait_for_terminal(&sender, &mut receiver).await {
                continue;
            }

            let _ignored: Either<Result<!, EndpointError>, Result<!, EndpointError>> = select(
                self.write_packets(&sender, &mut *render_buffer),
                self.read_packets(&mut receiver),
            )
            .await;
        }
    }
}

impl Log for UsbTerminalServer {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(mut self: &Self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.log.modify(|log| {
                let mut entry = log.push_back();
                Self::write_record(record, &mut entry);
                entry.build(LogLine::Log);
            });
        }
    }

    fn flush(&self) {}
}
impl UsbServer for UsbTerminalServer {
    type ConfigDescBuffer = [u8; 128];
    type BosDescBuffer = [u8; 16];
    type MsosDescBuffer = [u8; 256];
    fn build(
        &'static self,
        spawner: Spawner,
        builder: &mut Builder<'static, Driver<'static, USB>>,
    ) -> Result<(), Error> {
        let state = make_static!(State, State::new());
        let class = CdcAcmClass::new(builder, state, MAX_PACKET_SIZE as u16);

        let (sender, receiver, control) = class.split_with_control();
        let sender = make_static!(UsbSender, Mutex::new(sender));

        spawner.spawn({
            #[embassy_executor::task]
            async fn control_task(
                module: &'static UsbTerminalServer,
                sender: ControlChanged<'static>,
            ) {
                module.control(sender).await;
            }
            control_task(self, control)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn parse_task(module: &'static UsbTerminalServer, sender: &'static UsbSender) {
                module.parse(sender).await;
            }
            parse_task(self, sender)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn do_packets_task(
                server: &'static UsbTerminalServer,
                sender: &'static UsbSender,
                receiver: UsbReceiver,
            ) {
                server.do_packets(sender, receiver).await
            }
            do_packets_task(self, sender, receiver)?
        });
        Ok(())
    }
}
