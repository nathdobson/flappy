#![allow(unused_imports)]
#![allow(dead_code)]
#![deny(unused_must_use)]
#![allow(unused_variables)]
#![feature(never_type)]
#![allow(unreachable_code)]
#![feature(unsized_const_params)]
#![feature(adt_const_params)]
#![feature(unwrap_infallible)]
#![feature(async_fn_traits)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![allow(incomplete_features)]
#![allow(unused_mut)]
#![feature(unsized_fn_params)]
#![allow(internal_features)]
#![deny(non_snake_case)]

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
mod utils;
mod value_editor;
mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use crate::connect_tab::ConnectTab;
use crate::display::{Display, DisplayState};
use crate::error::Error;
use crate::home_tab::HomeTab;
use crate::mqtt_connector::run_mqtt;
use crate::query_params::{QueryParams, QueryParamsCell};
use crate::setup_tab::SetupTab;
use crate::status::{Status, StatusPriority};
use crate::tabs::{TabContainer, TabContent};
use crate::utils::{create_element, create_text_node, document, get_element_by_id, sleep};
use embassy_futures::select::{Either, Either4, Either5, select, select4, select5};
use futures_util::AsyncWriteExt;
use io_adapters::split::split_io;
use io_adapters::tokio::TokioStreamAdapter;
use log::{error, info, warn};
use protocol::display::{
    DISPLAY_REQUEST_CAPACITY, DisplayRequest, DisplayResponse, MAX_GLYPH_BYTES, MAX_GLYPHS,
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
use tokio::sync::mpsc::{Receiver, channel};
use tokio::try_join;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    HtmlDivElement, HtmlElement, HtmlFormElement, HtmlInputElement, Node, Window, window,
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
    let mut default = 0;
    if !query_params.borrow().ws_url.is_empty() {
        default = 1;
    }
    let tabs = TabContainer::new(vec![home, connect, setup], default, query_params)?;
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
