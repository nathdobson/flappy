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
mod event_listener;
mod mqtt_connector;
mod mqtt_form;
mod query_params;
pub mod root;
mod send_form;
mod status;
mod tabs;
mod utils;

use crate::display::{Display, DisplayState};
use crate::error::Error;
use crate::error::Error::NoneError;
use crate::mqtt_connector::run_mqtt;
use crate::mqtt_form::MqttForm;
use crate::query_params::{QueryParams, QueryParamsCell};
use crate::root::Root;
use crate::send_form::SendForm;
use crate::status::{Status, StatusPriority};
use crate::tabs::{TabContainer, TabContent};
use crate::utils::{
    create_element, sleep, spawn_local_joinable, try_create_div, try_create_text_node,
    try_document, try_get_element_by_id,
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

struct TestTab {
    title: &'static str,
    id: &'static str,
    node: HtmlDivElement,
}

impl TabContent for TestTab {
    fn title(&self) -> &str {
        self.title
    }

    fn id(&self) -> &str {
        self.id
    }

    fn handle_visible(&self, visible: bool) {}

    fn node(&self) -> &HtmlDivElement {
        &self.node
    }
}

async fn main() -> Result<(), Error> {
    let query_params = Rc::new(QueryParamsCell::new()?);
    let tab1 = Rc::new(TestTab {
        title: "Tab1",
        id: "tab1",
        node: {
            let div = try_create_div()?;
            div.append_child(&(try_create_text_node("tab1")?.into()))?;
            div
        },
    });
    let tab2 = Rc::new(TestTab {
        title: "Tab2",
        id: "tab2",
        node: {
            let div = try_create_div()?;
            div.append_child(&(try_create_text_node("tab2")?.into()))?;
            div
        },
    });
    let tabs = TabContainer::new(
        vec![
            //
            tab1, tab2,
        ],
        query_params,
    )?;
    try_document()?
        .body()
        .ok_or(NoneError)?
        .append_child(tabs.node())?;
    pending::<!>().await;
    // let status = Status::new()?;
    // let Err(e) = Root::new(status.clone()).await;
    // status.set(StatusPriority::Error, format!("{}", e));
    Ok(())
}

#[wasm_bindgen]
pub fn foo() {
    info!("HI");
}
