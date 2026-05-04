use crate::display::Display;
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::mqtt_connector::{DisplayResponseContainer, run_mqtt};
use crate::query_params::QueryParamsCell;
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::AppendChild;
use crate::utils::create_element;
use log::error;
use protocol::display::DisplayRequest;
use std::rc::Rc;
use std::str::FromStr;
use tokio::sync::mpsc::channel;
use unicode_segmentation::UnicodeSegmentation;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlDivElement, HtmlElement, HtmlFormElement, HtmlInputElement};
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

fn append_field(
    form: &HtmlFormElement,
    id: &str,
    text: &str,
    _placeholder: &str,
) -> Result<HtmlInputElement, Error> {
    let label = form.append_element::<"label">()?;
    label.append_text(text)?;
    label.set_html_for(id);
    let input = form.append_element::<"input">()?;
    input.set_type("text");
    input.set_id(id);
    Ok(input)
}

fn append_submit(form: &HtmlFormElement, text: &str) -> Result<HtmlInputElement, Error> {
    let _label = form.append_element::<"div">()?;
    let input = form.append_element::<"input">()?;
    input.set_id("submit");
    input.set_type("submit");
    input.set_value(text);
    Ok(input)
}

struct ConnectForm {
    form: HtmlFormElement,
    listener: EventListener<'static>,
}

struct Connect {
    status: Rc<Status>,
    submit_listener: EventListener<'static>,
    change_listener: EventListener<'static>,
}

impl ConnectTab {
    pub fn new(query_params: Rc<QueryParamsCell>) -> Result<Self, Error> {
        let node = create_element::<"div">()?;
        node.set_class_name("connect-tab");
        let inner;
        if query_params.borrow().ws_url.is_empty() {
            let form = node.append_element::<"form">()?;
            form.set_class_name("mqtt-form");
            let ws_url = append_field(
                &form,
                "ws_url",
                "MQTT WebSocket URL",
                "e.g. wss://my_mqq_host.com:8084/mqtt",
            )?;
            let username = append_field(&form, "username", "MQTT Username", "e.g. MyUsername")?;
            let password = append_field(
                &form,
                "password",
                "MQTT Password",
                "e.g. CorrectHorseBatteryStaple",
            )?;
            let topic = append_field(
                &form,
                "topic",
                "MQTT Topic",
                "e.g. display/name-of-my-display",
            )?;
            append_submit(&form, "Connect")?;

            inner = ConnectTabInner::ConnectForm(ConnectForm {
                form: form.clone(),
                listener: EventListener::new(&form, EventType::Submit, move |e| {
                    e.prevent_default();
                    if ws_url.value().is_empty() {
                        return;
                    }
                    if let Err(e) = query_params.modify(|query_params| {
                        query_params.ws_url = ws_url.value();
                        query_params.username = username.value();
                        query_params.password = password.value();
                        query_params.topic = topic.value();
                    }) {
                        error!("When connecting: {:?}", e);
                    }
                })?,
            });
        } else {
            let display = Display::new()?;
            node.append_child(display.node())?;
            let status = Status::new()?;
            node.append_child(status.node())?;
            let form = node.append_element::<"form">()?;
            form.set_class_name("content-form");
            let text = form.append_element::<"input">()?;
            text.set_type("text");
            text.set_placeholder("e.g. helloworld");
            let change_listener = EventListener::new(&text, EventType::Input, {
                let status = status.clone();
                let display = display.clone();
                let text = text.clone();
                move |e| {
                    if let Some(info) = display.info() {
                        status.set(
                            StatusPriority::Info,
                            format!(
                                "{} characters left",
                                (info.glyphs as isize)
                                    - (text.value().graphemes(true).count() as isize)
                            ),
                        );
                    }
                }
            })?;

            let submit = form.append_element::<"input">()?;
            submit.set_id("submit");
            submit.set_type("submit");
            submit.set_value("Send to Display");

            let (request_send, request_recv) = channel::<DisplayRequest>(10);
            let (response_send, response_recv) = channel::<DisplayResponseContainer>(10);

            let submit_listener = EventListener::new(&form, EventType::Submit, {
                let status = status.clone();
                let text = text.clone();
                move |e| {
                    e.prevent_default();
                    let msg = match heapless::String::from_str(&text.value()) {
                        Ok(msg) => msg,
                        Err(_) => {
                            status.set(StatusPriority::Error, "Message too long".to_string());
                            return;
                        }
                    };
                    if let Err(_) = request_send.try_send(DisplayRequest::Run(msg)) {
                        status.set(StatusPriority::Error, "Message queue overflow".to_string());
                    }
                }
            })?;

            spawn_local({
                let status = status.clone();
                async move {
                    let Err(e) =
                        run_mqtt(query_params, status.clone(), request_recv, response_send).await;
                    status.set_error(StatusPriority::Error, &e);
                }
            });

            spawn_local({
                let status = status.clone();
                let display = display.clone();
                async move {
                    let Err(e) = display.run_display(response_recv, status.clone()).await;
                    status.set_error(StatusPriority::Error, &e);
                }
            });

            inner = ConnectTabInner::Connect(Connect {
                status,
                submit_listener,
                change_listener,
            });
        }

        Ok(ConnectTab { node, inner })
    }
}
