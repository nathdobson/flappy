#![allow(unused_imports)]
#![allow(dead_code)]
#![deny(unused_must_use)]
#![allow(unused_variables)]
#![feature(never_type)]
#![allow(unreachable_code)]
#![feature(try_blocks)]
#![feature(unsized_const_params)]
#![feature(adt_const_params)]
#![feature(unwrap_infallible)]
#![allow(incomplete_features)]
mod error;
mod mqtt_socket;
mod utils;

use crate::error::Error;
use crate::mqtt_socket::run_mqtt;
use crate::utils::{create_element, sleep, try_create_div, try_document, try_get_element_by_id};
use arena::ArenaStorage;
use embassy_futures::select::{select, select4, select5, Either, Either4, Either5};
use io_adapter::split::split_io;
use io_adapter::tokio::TokioStreamAdapter;
use log::{error, info, warn};
use proto::MAX_GLYPH_BYTES;
use proto::{FlappyMessage, FlappyRequest, FlappyResponse};
use serde::{Deserialize, Serialize};
use std::future::pending;
use std::ops::Add;
use std::pin::pin;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::mpsc::channel;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, HtmlDivElement, HtmlFormElement, HtmlInputElement, Window};
use ws_stream_wasm::WsMeta;

#[wasm_bindgen(start)]
async fn start() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();
    if let Err(e) = main().await {
        error!("uncaught error: {:?}", e);
    }
}

struct Display {
    form: HtmlFormElement,
    input: HtmlInputElement,
    inners: Vec<HtmlDivElement>,
    dots: Vec<char>,
}

impl Display {
    pub fn new() -> Result<Self, Error> {
        let display: HtmlDivElement = try_get_element_by_id("display")?;
        let mut inners = vec![];
        for i in 0..10 {
            let letter_outer = create_element::<"div">()?;
            letter_outer.set_class_name("letter-outer");
            let letter_inner: HtmlDivElement = create_element::<"div">()?;
            letter_inner.set_class_name("letter-inner");
            letter_outer.append_child(&letter_inner)?;
            display.append_child(&letter_outer)?;
            inners.push(letter_inner);
        }
        let form: HtmlFormElement = try_get_element_by_id("form")?;
        let input: HtmlInputElement = try_get_element_by_id("content")?;
        let mut dots = vec![];
        for i in 0..8 {
            let mut codepoint = 0x2800;
            for k in 0..4 {
                let index = match (i + k) % 8 {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    3 => 6,
                    4 => 7,
                    5 => 5,
                    6 => 4,
                    7 => 3,
                    _ => unreachable!(),
                };
                codepoint |= 1 << index;
            }
            dots.push(char::from_u32(codepoint).unwrap());
        }
        Ok(Display {
            form,
            input,
            inners,
            dots,
        })
    }
    // pub fn start(&self) {
    //     for inner in &self.inners {
    //         inner.set_text_content(Some("⋮"));
    //     }
    // }
    // pub fn stop(&self, text: &[heapless::String<MAX_GLYPH_BYTES>]) {
    //     for (inner, glyph) in self.inners.iter().zip(text.iter()) {
    //         inner.set_text_content(Some(glyph));
    //     }
    // }
    pub fn set_on_submit(&self, mut on_submit: impl FnMut(&str)) {
        let closure = Closure::wrap(Box::new(move || {
            on_submit(&self.input.value());
            false
        }) as Box<dyn FnMut() -> bool>);
        self.form
            .set_onsubmit(Some(&closure.as_ref().unchecked_ref()));
        closure.forget();
    }
    pub async fn handle_response(&self, resp: FlappyResponse) -> Result<!, Error> {
        match resp {
            FlappyResponse::Start(_) => {
                for step in 0.. {
                    for inner in &self.inners {
                        inner.set_text_content(Some(&format!(
                            "{}",
                            self.dots[step % self.dots.len()]
                        )));
                    }
                    sleep(100).await;
                }
            }
            FlappyResponse::Stop(text) => {
                for (index, inner) in self.inners.iter().enumerate() {
                    inner.set_text_content(Some(text.get(index).map_or(" ", |x| &**x)));
                }
            }
        }
        pending().await
    }
}

async fn main() -> Result<(), Error> {
    let display = Display::new()?;
    let (request_send, request_recv) = channel::<FlappyRequest>(10);
    let (response_send, mut response_recv) = channel::<FlappyResponse>(10);
    display.set_on_submit(|value| {
        if let Err::<(), Error>(e) =
            try { request_send.try_send(FlappyRequest::Run(heapless::String::from_str(value)?))? }
        {
            error!("{:?}", e);
        }
    });
    spawn_local(async move {
        let mut response = FlappyResponse::Stop(heapless::Vec::new());
        loop {
            match select(response_recv.recv(), display.handle_response(response)).await {
                Either::First(None) => return,
                Either::First(Some(new)) => response = new,
                Either::Second(x) => {
                    error!("{:?}", x.into_err());
                    return;
                }
            }
        }
    });
    run_mqtt(request_recv, response_send).await?;
    Ok(())
}

#[wasm_bindgen]
pub fn foo() {
    info!("HI");
}
