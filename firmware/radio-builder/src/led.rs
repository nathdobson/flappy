use cyw43::Control;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;

pub struct RadioLed {
    pub(crate) control: &'static Mutex<NoopRawMutex, Control<'static>>,
}

impl RadioLed {
    pub async fn set_led(&self, value: bool) {
        self.control.lock().await.gpio_set(0, value).await;
    }
}
