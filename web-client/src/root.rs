use crate::display::{Display, DisplayState};
use crate::error::Error;
use crate::mqtt_connector::run_mqtt;
use crate::mqtt_form::MqttForm;
use crate::query_params::{QueryParams, QueryParamsCell};
use crate::send_form::SendForm;
use crate::status::{Status, StatusPriority};
use crate::utils::spawn_local_joinable;
use embassy_futures::select::{select, Either};
use log::error;
use protocol::display::{DisplayRequest, DisplayResponse, DISPLAY_REQUEST_CAPACITY, MAX_GLYPHS};
use protocol::setup::DeviceInfo;
use std::iter;
use std::rc::Rc;
use std::str::FromStr;
use tokio::sync::mpsc::{channel, Receiver};
use tokio::try_join;

pub struct Root {
    send_form: SendForm,
    status: Rc<Status>,
}

pub enum DisplayResponseContainer {
    DisplayResponse(DisplayResponse),
    DeviceInfo(DeviceInfo),
}

impl Root {
    pub async fn new(status: Rc<Status>) -> Result<!, Error> {
        let params = Rc::new(QueryParamsCell::new()?);
        let mut display = Display::new()?;
        let mut send_form = SendForm::new(params.borrow().spindle)?;
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
