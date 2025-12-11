#![allow(unused_imports)]
#![allow(dead_code)]
#![deny(unused_must_use)]
#![allow(unused_variables)]
#![feature(never_type)]
#![allow(unreachable_code)]
#![feature(try_blocks)]
mod error;

use crate::error::Error;
use arena::ArenaStorage;
use embassy_futures::select::{select, select4, select5, Either, Either4, Either5};
use io_adapter::split::split_io;
use io_adapter::tokio::TokioStreamAdapter;
use log::{error, info, warn};
use mqtt::proto::{Packet, Qos};
use mqtt::receiver::MqttReceiver;
use mqtt::sender::{ConnectRequest, MqttSender, PublishRequest};
use proto::{FlappyMessage, FlappyRequest};
use serde::{Deserialize, Serialize};
use std::future::pending;
use std::ops::Add;
use std::pin::pin;
use std::str::FromStr;
use std::time::Duration;
use url::Url;
use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlDivElement, HtmlFormElement, HtmlInputElement, Window};
use ws_stream_wasm::WsMeta;

const KEEPALIVE: u16 = 60;

#[wasm_bindgen(start)]
async fn start() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();
    if let Err(e) = main().await {
        error!("uncaught error: {:?}", e);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlappyQueryParams {
    ws_url: String,
    username: String,
    password: String,
    topic: String,
}

pub async fn sleep(millis: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis)
            .unwrap();
    };
    let p = js_sys::Promise::new(&mut cb);
    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}

fn create_display() -> Result<(), Error> {
    let window = web_sys::window().ok_or(Error::NoneError)?;
    let document = window.document().ok_or(Error::NoneError)?;
    let display: HtmlDivElement = document
        .get_element_by_id("display")
        .ok_or(Error::NoneError)?
        .dyn_into()
        .ok()
        .ok_or(Error::TypeError)?;
    for i in 0..10 {
        let letter_outer = document.create_element("div")?;
        letter_outer.set_class_name("letter-outer");
        let letter_inner = document.create_element("div")?;
        letter_inner.set_class_name("letter-inner");
        letter_inner.set_text_content(Some(&format!("{}", i)));
        letter_outer.append_child(&letter_inner)?;
        display.append_child(&letter_outer)?;
    }
    Ok(())
}

async fn main() -> Result<(), Error> {
    create_display()?;
    // return Ok(());
    let window = web_sys::window().ok_or(Error::NoneError)?;
    let document = window.document().ok_or(Error::NoneError)?;
    let form: HtmlFormElement = document
        .get_element_by_id("form")
        .ok_or(Error::NoneError)?
        .dyn_into()
        .ok()
        .ok_or(Error::TypeError)?;
    let input: HtmlInputElement = document
        .get_element_by_id("content")
        .ok_or(Error::NoneError)?
        .dyn_into()
        .ok()
        .ok_or(Error::TypeError)?;
    let (request_send, mut request_recv) = tokio::sync::mpsc::channel::<FlappyRequest>(10);
    let closure = Closure::wrap(Box::new(move || {
        info!("OnSubmit");
        if let Err::<(), Error>(e) = try {
            request_send.try_send(FlappyRequest::Run(heapless::String::from_str(
                &*input.value(),
            )?))?
        } {
            error!("{:?}", e);
        }
        false
    }) as Box<dyn FnMut() -> bool>);
    form.set_onsubmit(Some(&closure.as_ref().unchecked_ref()));
    closure.forget();
    let search = window.location().search()?;
    let search = search.strip_prefix("?").unwrap_or(&search);
    let params: FlappyQueryParams = serde_qs::from_str(&search)?;
    let (meta, stream) = WsMeta::connect(&params.ws_url, Some(vec!["mqtt"])).await?;
    let (read, write) = split_io(stream.into_io());
    let sender = MqttSender::<_, 1024, 1, 1>::new(TokioStreamAdapter(write));
    let mut receiver = MqttReceiver::new(TokioStreamAdapter(read));
    match select5(
        async {
            let mut arena = ArenaStorage::<1024>::new();
            loop {
                let (ack, packet) = receiver.receive(arena.start()).await?;
                match packet {
                    Packet::Publish(publish) => {
                        match serde_json_core::from_slice::<FlappyMessage>(&publish.payload) {
                            Ok((m, _)) => match m {
                                FlappyMessage::Request(_) => {}
                                FlappyMessage::Response(response) => {
                                    let status = document.get_element_by_id("display").unwrap();
                                    status.set_text_content(Some(&format!("{:?}", response)));
                                }
                            },
                            Err(e) => {
                                error!("Could not parse message: {:?}", e);
                            }
                        }
                    }
                    _ => {}
                }
                sender.acknowledge(ack)?;
            }
            Ok::<!, Error>(unreachable!())
        },
        async {
            sender.send_acks().await?;
            Ok::<!, Error>(unreachable!())
        },
        async {
            let disconnect = sender.wait_disconnect().await?;
            Err(Error::Disconnect(disconnect))
        },
        async {
            sleep(KEEPALIVE as i32 * 1000).await;
            loop {
                let mut timer = pin!(sleep(KEEPALIVE as i32 * 1000));
                match select(&mut timer, sender.ping()).await {
                    Either::First(()) => return Err(Error::DeadlineExceeded),
                    Either::Second(p) => p?,
                }
                timer.await
            }
            Ok::<!, Error>(unreachable!())
        },
        async {
            let client_id = "flappy_web";
            info!(
                "Connecting to broker with client_id '{}' and username '{}'",
                client_id, params.username
            );
            sender
                .connect(&ConnectRequest {
                    client_id,
                    username: Some(&params.username),
                    password: Some(&params.password),
                    keepalive: 0,
                })
                .await?;
            info!("Connected to broker");
            info!("Subscribing to {}", params.topic);
            sender.subscribe(&params.topic).await?;
            info!("Subscribed");
            while let Some(next) = request_recv.recv().await {
                info!("Publishing {:?}", next);
                sender
                    .publish(&PublishRequest {
                        qos: Qos::AtMostOnce,
                        topic: &params.topic,
                        payload: &serde_json_core::to_vec::<_, 1024>(&FlappyMessage::Request(
                            next,
                        ))?,
                    })
                    .await?;
                info!("Published");
            }
            Ok::<!, Error>(unreachable!())
        },
    )
    .await
    {
        Either5::First(x) => x?,
        Either5::Second(x) => x?,
        Either5::Third(x) => x?,
        Either5::Fourth(x) => x?,
        Either5::Fifth(x) => x?,
    }
    Ok(())
}

#[wasm_bindgen]
pub fn foo() {
    info!("HI");
}
