use crate::ble::BleClientBuilder;
use futures_util::StreamExt;
use protocol::setup::SetupRequest;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test() {
    timeout(Duration::from_secs(30), async {
        let mut stream = BleClientBuilder::scan().await.unwrap();
        let device = loop {
            let device = stream.next().await.expect("end of stream").unwrap();
            if device.address() == "2b1eb34b-169f-508f-66f1-d5cb71cbc604" {
                break device;
            } else {
                println!("ignoring {}", device.address());
            }
        };
        println!("connecting {}", device.address());
        let mut device = device.connect().await.unwrap();
        println!("connected");
    })
    .await
    .unwrap();
}
