use crate::error::Error;

pub async fn list_devices() -> Result<Vec<crate::DeviceInfo>, Error> {
    Ok(nusb::list_devices()
        .await?
        .map(|device_info| crate::DeviceInfo(crate::platform::DeviceInfo { device_info }))
        .collect())
}
