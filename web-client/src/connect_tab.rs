use crate::display::{Display, DisplayState};
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::mqtt_connector::run_mqtt;
use crate::query_params::QueryParamsCell;
use crate::root::DisplayResponseContainer;
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::AppendChild;
use crate::utils::{create_element, spawn_local_joinable};
use embassy_futures::select::{select, Either};
use log::error;
use protocol::display::{DisplayRequest, DisplayResponse, MAX_GLYPHS};
use std::{iter, mem};
use std::rc::Rc;
use std::str::FromStr;
use tokio::sync::mpsc::{channel, Receiver};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlDivElement, HtmlElement, HtmlFormElement, HtmlInputElement, Node};

enum ConnectTabInner {
    ConnectForm(ConnectForm),
    Connect(Connect),
}

pub struct ConnectTab {
    node: HtmlDivElement,
    inner: ConnectTabInner,
}

impl TabContent for ConnectTab {
    fn title(&self) -> &str {
        "Connect to Display"
    }
    fn id(&self) -> &str {
        "connect"
    }
    fn node(&self) -> &HtmlElement {
        &self.node
    }
}

fn append_field(form: &HtmlFormElement, id: &str, text: &str) -> Result<HtmlInputElement, Error> {
    let label = form.append_element::<"label">()?;
    label.append_text(text)?;
    label.set_html_for(id);
    let input = form.append_element::<"input">()?;
    input.set_type("text");
    input.set_id(id);
    Ok(input)
}

struct ConnectForm {
    form: HtmlFormElement,
    listener: EventListener<'static>,
}

struct Connect {
    status: Rc<Status>,
}

impl ConnectTab {
    pub fn new(query_params: Rc<QueryParamsCell>) -> Result<Self, Error> {
        let node = create_element::<"div">()?;
        let inner;
        if query_params.borrow().ws_url.is_empty() {
            let form = node.append_element::<"form">()?;
            form.set_class_name("mqtt-form");
            let ws_url = append_field(&form, "ws_url", "MQTT WebSocket URL")?;
            let username = append_field(&form, "username", "MQTT Username")?;
            let password = append_field(&form, "password", "MQTT Password")?;
            let topic = append_field(&form, "topic", "MQTT Topic")?;
            let connect_label = form.append_element::<"label">()?;
            connect_label.append_text("Connect")?;
            connect_label.set_html_for("connect");
            let connect_input = form.append_element::<"input">()?;
            connect_input.set_id("connect");
            connect_input.set_type("submit");
            connect_input.set_value("Connect");

            inner = ConnectTabInner::ConnectForm(ConnectForm {
                form: form.clone(),
                listener: EventListener::new(form.clone().into(), EventType::Submit, move |e| {
                    e.prevent_default();
                    if ws_url.value().is_empty() {
                        return false;
                    }
                    if let Err(e) = query_params.modify(|query_params| {
                        query_params.ws_url = ws_url.value();
                        query_params.username = username.value();
                        query_params.password = password.value();
                        query_params.topic = topic.value();
                    }) {
                        error!("When connecting: {:?}", e);
                    }
                    false
                })?,
            });
        } else {
            let status = Status::new()?;
            node.append_child(status.node())?;
            let display = Display::new()?;
            node.append_child(display.node())?;

            let (request_send, request_recv) = channel::<DisplayRequest>(10);
            let (response_send, mut response_recv) = channel::<DisplayResponseContainer>(10);

            spawn_local({
                let status = status.clone();
                async move {
                    let Err(e) =
                        run_mqtt(query_params, status.clone(), request_recv, response_send).await;
                    status.set(StatusPriority::Error, format!("{}", e));
                }
            });

            spawn_local({
                let status = status.clone();
                async move {
                    let Err(e) = display.run_display(response_recv, status.clone()).await;
                    status.set(StatusPriority::Error, format!("{}", e));
                }
            });

            mem::forget(request_send);

            inner = ConnectTabInner::Connect(Connect { status });
        }

        Ok(ConnectTab { node, inner })
    }
}
