use log::info;
use crate::error::Error;
use crate::utils::{bluetooth, try_window};

pub const BROWSER_SUPPORT_MESSAGE: &str = "
Bluetooth and USB connectivity require a supported web browser and platform.
<ul>
    <li>
        Browser support:
        <ul>
          <li>✅ Chrome</li>
          <li>✅ Edge</li>
          <li>✅ Opera</li>
          <li>❌ Safari</li>
          <li>❌ Firefox</li>
        </ul>
    </li>
    <li>
        Platform support:
        <ul>
          <li>✅ macOS</li>
          <li>✅ Windows</li>
          <li>✅ Linux</li>
          <li>✅ ChromeOS</li>
          <li>✅ Android</li>
          <li>❌ iOS</li>
        </ul>
    </li>
</ul>
";


pub fn check_usb_supported() -> Result<(), Error> {
    let usb = try_window()?
        .navigator()
        .usb()
        .ok_or(Error::UsbNotSupported)?;
    Ok(())
}

pub fn check_ble_supported() -> Result<(), Error>{
    bluetooth()?;
    Ok(())
}