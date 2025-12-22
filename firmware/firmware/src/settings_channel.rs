use core::cell::{Ref, RefCell, RefMut};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use protocol::setup::AppSettings;

pub struct SettingsChannel {
    settings: RefCell<AppSettings>,
    all: Signal<NoopRawMutex, ()>,
    mqtt: Signal<NoopRawMutex, ()>,
    wifi: Signal<NoopRawMutex, ()>,
    display: Signal<NoopRawMutex, ()>,
}

impl SettingsChannel {
    pub fn new(settings: AppSettings) -> Self {
        SettingsChannel {
            settings: RefCell::new(settings),
            all: Signal::new(),
            mqtt: Signal::new(),
            wifi: Signal::new(),
            display: Signal::new(),
        }
    }
    pub fn read(&self) -> AppSettings {
        self.settings.borrow().clone()
    }
}
