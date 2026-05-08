use crate::error::Error;
use crate::utils::{bluetooth, try_window};
use regex::Regex;

pub fn force_webview_to_chrome() -> Result<(), Error> {
    let window = try_window()?;
    let location = window.location();
    let is_webview = Regex::new(r#"Version\/\d+.*\/\d+.0.0.0 Mobile|; ?wv"#)
        .unwrap()
        .is_match(&window.navigator().user_agent()?);
    if is_webview {
        location.set_href(&format!("intent:{}#Intent;end", location.href()?))?;
    }
    Ok(())
}

pub const BROWSER_SUPPORT_MESSAGE: &str = "
<b>
    Bluetooth and USB connectivity require a supported web browser and platform.
    <ul>
        <li>
            Browser support:
            <ul>
              <li>✅ Chrome</li>
              <li>✅ Edge</li>
              <li>✅ Opera</li>
              <li>❌ Android WebView</li>
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
</b>
";

pub fn check_usb_supported() -> Result<(), Error> {
    try_window()?
        .navigator()
        .usb()
        .ok_or(Error::UsbNotSupported)?;
    Ok(())
}

pub fn check_ble_supported() -> Result<(), Error> {
    bluetooth()?;
    Ok(())
}
