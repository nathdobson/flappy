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
#![allow(unused_mut)]

mod display;
mod error;
mod mqtt_connector;
mod mqtt_form;
mod query_params;
mod send_form;
mod status;
mod utils;

use crate::display::{Display, DisplayState};
use crate::error::Error;
use crate::mqtt_connector::run_mqtt;
use crate::mqtt_form::MqttForm;
use crate::query_params::FlappyQueryParams;
use crate::send_form::SendForm;
use crate::status::{Status, StatusPriority};
use crate::utils::{
    create_element, sleep, spawn_local_joinable, try_create_div, try_document,
    try_get_element_by_id,
};
use embassy_futures::select::{select, select4, select5, Either, Either4, Either5};
use futures_util::AsyncWriteExt;
use io_adapters::split::split_io;
use io_adapters::tokio::TokioStreamAdapter;
use log::{error, info, warn};
use protocol::display::{
    DisplayRequest, DisplayResponse, DISPLAY_REQUEST_CAPACITY, MAX_GLYPHS, MAX_GLYPH_BYTES,
};
use protocol::setup::DeviceInfo;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::future::pending;
use std::iter;
use std::ops::Add;
use std::pin::pin;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{channel, Receiver};
use tokio::try_join;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, HtmlDivElement, HtmlElement, HtmlFormElement, HtmlInputElement, Window};
use ws_stream_wasm::WsMeta;

#[wasm_bindgen(start)]
async fn start() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();
    if let Err(e) = main().await {
        error!("uncaught error: {:?}", e);
    }
}

pub struct Root {
    send_form: SendForm,
    status: Rc<Status>,
}

enum DisplayResponseContainer {
    DisplayResponse(DisplayResponse),
    DeviceInfo(DeviceInfo),
}

impl Root {
    pub async fn new(status: Rc<Status>) -> Result<!, Error> {
        let params = FlappyQueryParams::new()?;
        let mut display = Display::new()?;
        let mut send_form = SendForm::new(params.spindle)?;
        let (request_send, request_recv) = channel::<DisplayRequest>(10);
        let (response_send, mut response_recv) = channel::<DisplayResponseContainer>(10);
        send_form.set_on_submit(|value| {
            let mut value: String = value.to_owned();
            value.truncate(DISPLAY_REQUEST_CAPACITY);
            match heapless::String::from_str(&value) {
                Err(e) => {
                    error!("{:?}", e);
                }
                Ok(value) => match request_send.try_send(DisplayRequest::Run(value)) {
                    Ok(()) => {}
                    Err(e) => {
                        error!("{:?}", e);
                    }
                },
            }
        });
        send_form.set_on_submit_src(|value| {
            let mut value: String = value.to_owned();
            if value.len() > DISPLAY_REQUEST_CAPACITY {
                error!("code too long");
                return;
            }
            match heapless::String::from_str(&value) {
                Err(e) => {
                    error!("{:?}", e);
                }
                Ok(value) => match request_send.try_send(DisplayRequest::RunSpindle(value)) {
                    Ok(()) => {}
                    Err(e) => {
                        error!("{:?}", e);
                    }
                },
            }
        });

        let mqtt_form = MqttForm::new(&params)?;
        let this = Rc::new(Root {
            send_form,
            status: status.clone(),
        });

        try_join! {
            spawn_local_joinable(this.clone().run_display(status.clone(),display,response_recv)).try_join(),
            spawn_local_joinable(run_mqtt(params,status.clone(),request_recv, response_send)).try_join(),
        }?;
        todo!();
    }
    async fn run_display(
        self: Rc<Self>,
        status: Rc<Status>,
        mut display: Display,
        mut response_recv: Receiver<DisplayResponseContainer>,
    ) -> Result<!, Error> {
        let mut state = DisplayState::Stopped(
            iter::repeat_n(heapless::String::from_str(" ").unwrap(), MAX_GLYPHS).collect(),
        );
        loop {
            match select(response_recv.recv(), display.handle_state(state.clone())).await {
                Either::First(None) => return Err(Error::UnexpectedEof),
                Either::First(Some(new)) => match new {
                    DisplayResponseContainer::DisplayResponse(response) => match response {
                        DisplayResponse::Start(_) => state = DisplayState::Running,
                        DisplayResponse::Stop(text) => state = DisplayState::Stopped(text),
                    },
                    DisplayResponseContainer::DeviceInfo(info) => {
                        status.set(StatusPriority::Info, "Connected!".to_string());
                        display.build(&info).unwrap_or_else(|e| error!("{:?}", e))
                    }
                },
                Either::Second(e) => return e,
            }
        }
    }
}

async fn main() -> Result<(), Error> {
    let status = Status::new()?;
    let Err(e) = Root::new(status.clone()).await;
    status.set(StatusPriority::Error, format!("{}", e));
    Ok(())
}

#[wasm_bindgen]
pub fn foo() {
    info!("HI");
}
