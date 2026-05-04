#![allow(incomplete_features, internal_features)]
#![deny(unused_must_use, non_snake_case)]
#![feature(
    never_type,
    unsized_const_params,
    adt_const_params,
    unwrap_infallible,
    async_fn_traits,
    tuple_trait,
    unboxed_closures,
    unsized_fn_params,
    trim_prefix_suffix
)]
mod bind_weak;
mod connect_tab;
mod connection;
mod display;
mod dyn_async_fn;
mod error;
mod event_listener;
mod home_tab;
mod mqtt_connector;
mod query_params;
mod setup_tab;
mod status;
mod tabs;
//mod usb_connection;
mod browser_support;
mod firmware_tab;
mod utils;
mod value_editor;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use crate::connect_tab::ConnectTab;
use crate::display::{Display, DisplayState};
use crate::error::Error;
use crate::firmware_tab::FirmwareTab;
use crate::home_tab::HomeTab;
use crate::mqtt_connector::run_mqtt;
use crate::query_params::{QueryParams, QueryParamsCell};
use crate::setup_tab::SetupTab;
use crate::status::{Status, StatusPriority};
use crate::tabs::{TabContainer, TabContent};
use crate::utils::{create_element, create_text_node, document, get_element_by_id, sleep};
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
use web_sys::{
    window, HtmlDivElement, HtmlElement, HtmlFormElement, HtmlInputElement, Node, Window,
};
use ws_stream_wasm::WsMeta;

#[wasm_bindgen(start)]
async fn start() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();
    if let Err(e) = main().await {
        error!("uncaught error: {:?}", e);
    }
}
async fn main() -> Result<(), Error> {
    let query_params = Rc::new(QueryParamsCell::new()?);
    let home = Rc::new(HomeTab::new()?);
    let connect = Rc::new(ConnectTab::new(query_params.clone())?);
    let setup = SetupTab::new()?;
    let firmware = FirmwareTab::new()?;
    let mut default = 0;
    if !query_params.borrow().ws_url.is_empty() {
        default = 1;
    }
    let tabs = TabContainer::new(vec![home, connect, setup, firmware], default, query_params)?;
    document()?
        .body()
        .ok_or(Error::CannotFindElement)?
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
