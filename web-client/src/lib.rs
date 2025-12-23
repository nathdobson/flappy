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
use io_adapters::split::split_io;
use io_adapters::tokio::TokioStreamAdapter;
use log::{error, info, warn};
use protocol::display::{
    DisplayMessage, DisplayRequest, DisplayResponse, DISPLAY_REQUEST_CAPACITY, MAX_GLYPHS,
    MAX_GLYPH_BYTES,
};
use protocol::setup::DeviceInfo;
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

#[derive(Clone)]
enum State {
    Running,
    Stopped(heapless::Vec<heapless::String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
}

impl Display {
    pub fn new() -> Result<Self, Error> {
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
            inners: vec![],
            dots,
        })
    }
    pub fn set_on_submit(&self, mut on_submit: impl FnMut(&str)) {
        let closure = Closure::wrap(Box::new(move || {
            on_submit(&self.input.value());
            false
        }) as Box<dyn FnMut() -> bool>);
        self.form
            .set_onsubmit(Some(&closure.as_ref().unchecked_ref()));
        closure.forget();
    }
    pub async fn handle_state(&self, resp: State) -> Result<!, Error> {
        match resp {
            State::Running => {
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
            State::Stopped(text) => {
                for (index, inner) in self.inners.iter().enumerate() {
                    inner.set_text_content(Some(text.get(index).map_or(" ", |x| &**x)));
                }
            }
        }
        pending().await
    }
    pub fn build(&mut self, info: &DeviceInfo) -> Result<(), Error> {
        info!("DeviceInfo = {:?}", info);
        let display: HtmlDivElement = try_get_element_by_id("display")?;
        display
            .style()
            .set_property("color", &format!("#{}", info.foreground))?;
        let mut inners = vec![];
        for i in 0..info.glyphs {
            let letter_outer = create_element::<"div">()?;
            letter_outer.set_class_name("letter-outer");
            letter_outer
                .style()
                .set_property("background", &format!("#{}", info.background))?;
            let letter_inner: HtmlDivElement = create_element::<"div">()?;
            letter_inner
                .style()
                .set_property("color", &format!("#{}", info.foreground))?;
            letter_outer
                .style()
                .set_property("color", &format!("#{}", info.foreground))?;
            letter_inner.set_class_name("letter-inner");

            letter_outer.append_child(&letter_inner)?;
            display.append_child(&letter_outer)?;
            inners.push(letter_inner);
        }
        self.inners = inners;
        Ok(())
    }
}

async fn main() -> Result<(), Error> {
    let mut display = Display::new()?;
    let (request_send, request_recv) = channel::<DisplayRequest>(10);
    let (response_send, mut response_recv) = channel::<DisplayResponse>(10);
    request_send.send(DisplayRequest::DeviceInfo).await?;
    display.set_on_submit(|value| {
        let mut value: String = value.to_owned();
        value.truncate(DISPLAY_REQUEST_CAPACITY);
        if let Err::<(), Error>(e) =
            try { request_send.try_send(DisplayRequest::Run(heapless::String::from_str(&value)?))? }
        {
            error!("{:?}", e);
        }
    });
    spawn_local(async move {
        let mut state = State::Stopped(heapless::Vec::new());
        loop {
            match select(response_recv.recv(), display.handle_state(state.clone())).await {
                Either::First(None) => return,
                Either::First(Some(new)) => {
                    //
                    match new {
                        DisplayResponse::Start(_) => state = State::Running,
                        DisplayResponse::Stop(text) => state = State::Stopped(text),
                        DisplayResponse::DeviceInfo(info) => {
                            display.build(&info).unwrap_or_else(|e| error!("{:?}", e))
                        }
                    }
                }
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
