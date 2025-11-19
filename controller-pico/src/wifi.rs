use crate::secrets::{WIFI_NETWORK, WIFI_PASSWORD};
use core::str::from_utf8;
use cyw43::{Control, JoinOptions, NetDriver};
use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config, StackResources};
use embassy_time::{Duration, Timer};
use log::{error, info};
use rand_core::RngCore;
use reqwless::client::HttpClient;
use reqwless::request::Method;
use serde::Deserialize;
use serde_json_core::from_slice;
use static_cell::StaticCell;

pub struct WifiModuleBuilder<'build, R> {
    spawner: Spawner,
    rng: &'build mut R,
    net_device: NetDriver<'static>,
    control: Control<'static>,
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

pub struct WifiModule {}

impl<'build, R: RngCore> WifiModuleBuilder<'build, R> {
    #[must_use]
    pub async fn build(mut self) -> WifiModule {
        let config = Config::dhcpv4(Default::default());
        let seed = self.rng.next_u64();

        // Init network stack
        static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
        let (stack, runner) = embassy_net::new(
            self.net_device,
            config,
            RESOURCES.init(StackResources::new()),
            seed,
        );

        self.spawner.spawn(net_task(runner)).unwrap();

        while let Err(err) = self
            .control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD))
            .await
        {
            info!("join failed with status={}", err.status);
        }

        info!("waiting for link...");
        stack.wait_link_up().await;

        info!("waiting for DHCP...");
        stack.wait_config_up().await;

        // And now we can use it!
        info!("Stack is up!");

        // And now we can use it!

        loop {
            let mut rx_buffer = [0; 4096];
            // Uncomment these for TLS requests:
            // let mut tls_read_buffer = [0; 16640];
            // let mut tls_write_buffer = [0; 16640];

            let client_state = TcpClientState::<1, 4096, 4096>::new();
            let tcp_client = TcpClient::new(stack, &client_state);
            let dns_client = DnsSocket::new(stack);
            // Uncomment these for TLS requests:
            // let tls_config = TlsConfig::new(seed, &mut tls_read_buffer, &mut tls_write_buffer, TlsVerify::None);

            // Using non-TLS HTTP for this example
            let mut http_client = HttpClient::new(&tcp_client, &dns_client);
            let url = "http://httpbin.org/json";
            // For TLS requests, use this instead:
            // let mut http_client = HttpClient::new_with_tls(&tcp_client, &dns_client, tls_config);
            // let url = "https://httpbin.org/json";

            info!("connecting to {}", &url);

            let mut request = match http_client.request(Method::GET, url).await {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to make HTTP request: {:?}", e);
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let response = match request.send(&mut rx_buffer).await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to send HTTP request: {:?}", e);
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("Response status: {}", response.status.0);

            let body_bytes = match response.body().read_to_end().await {
                Ok(b) => b,
                Err(_e) => {
                    error!("Failed to read response body");
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let body = match from_utf8(body_bytes) {
                Ok(b) => b,
                Err(_e) => {
                    error!("Failed to parse response body as UTF-8");
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            };
            info!("Response body length: {} bytes", body.len());

            // Parse the JSON response from httpbin.org/json
            #[derive(Deserialize)]
            struct SlideShow<'a> {
                author: &'a str,
                title: &'a str,
            }

            #[derive(Deserialize)]
            struct HttpBinResponse<'a> {
                #[serde(borrow)]
                slideshow: SlideShow<'a>,
            }

            let bytes = body.as_bytes();
            match from_slice::<HttpBinResponse>(bytes) {
                Ok((output, _used)) => {
                    info!("Successfully parsed JSON response!");
                    info!("Slideshow title: {:?}", output.slideshow.title);
                    info!("Slideshow author: {:?}", output.slideshow.author);
                }
                Err(e) => {
                    error!("Failed to parse JSON response: {:?}", e);
                    // Log preview of response for debugging
                    let preview = if body.len() > 200 { &body[..200] } else { body };
                    info!("Response preview: {:?}", preview);
                }
            }

            Timer::after(Duration::from_secs(1000)).await;
        }
    }
}
