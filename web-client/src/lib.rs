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
use crate::error::Error;
use crate::firmware_tab::FirmwareTab;
use crate::home_tab::HomeTab;
use crate::query_params::QueryParamsCell;
use crate::setup_tab::SetupTab;
use crate::tabs::TabContainer;
use crate::utils::document;
use log::{error, info};
use std::future::pending;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
async fn start() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();
    let Err(e) = main().await;
    error!("uncaught error: {:?}", e);
}
async fn main() -> Result<!, Error> {
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
}

#[wasm_bindgen]
pub fn foo() {
    info!("HI");
}
