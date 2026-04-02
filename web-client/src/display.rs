use crate::error::Error;
use crate::mqtt_connector::DisplayResponseContainer;
use crate::status::{Status, StatusPriority};
use crate::utils::{create_element, get_element_by_id, sleep};
use embassy_futures::select::{select, Either};
use log::{error, info};
use protocol::display::{
    DisplayRequest, DisplayResponse, DISPLAY_REQUEST_CAPACITY, MAX_GLYPHS, MAX_GLYPH_BYTES,
};
use protocol::setup::DeviceInfo;
use std::cell::{Ref, RefCell};
use std::future::pending;
use std::iter;
use std::rc::Rc;
use std::str::FromStr;
use tokio::sync::mpsc::Receiver;
use web_sys::{HtmlDivElement, HtmlElement};

pub struct DeviceState {
    inners: Vec<HtmlDivElement>,
    outers: Vec<HtmlDivElement>,
    info: Option<DeviceInfo>,
}

pub struct Display {
    display: HtmlDivElement,
    dots: Vec<char>,
    state: RefCell<DeviceState>,
}

#[derive(Clone)]
pub enum DisplayState {
    Running,
    Stopped(heapless::Vec<heapless::String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
}

impl Display {
    pub fn new() -> Result<Rc<Self>, Error> {
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
        let display = create_element::<"div">()?;
        display.set_class_name("display");
        Ok(Rc::new(Display {
            display,
            dots,
            state: RefCell::new(DeviceState {
                inners: vec![],
                outers: vec![],
                info: None,
            }),
        }))
    }
    pub fn node(&self) -> &HtmlElement {
        &self.display
    }
    pub fn info(&self) -> Option<Ref<'_, DeviceInfo>> {
        Ref::filter_map(self.state.borrow(), |x| x.info.as_ref()).ok()
    }
    pub async fn handle_state(&self, resp: DisplayState) -> Result<!, Error> {
        match resp {
            DisplayState::Running => {
                for step in 0.. {
                    for inner in &self.state.borrow().inners {
                        inner.set_text_content(Some(&format!(
                            "{}",
                            self.dots[step % self.dots.len()]
                        )));
                    }
                    sleep(100).await;
                }
            }
            DisplayState::Stopped(text) => {
                for (index, inner) in self.state.borrow().inners.iter().enumerate() {
                    inner.set_text_content(Some(text.get(index).map_or(" ", |x| &**x)));
                }
            }
        }
        pending().await
    }
    pub fn build(&self, info: &DeviceInfo) -> Result<(), Error> {
        let mut state = self.state.borrow_mut();
        state.info = Some(info.clone());
        info!("DeviceInfo = {:?}", info);
        self.display
            .style()
            .set_property("color", &format!("#{}", info.foreground))?;
        state.inners.clear();
        for outer in state.outers.drain(..) {
            self.display.remove_child(&outer)?;
        }
        let mut inners = vec![];
        let mut outers = vec![];
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
            self.display.append_child(&letter_outer)?;
            inners.push(letter_inner);
            outers.push(letter_outer);
        }
        state.inners = inners;
        state.outers = outers;
        Ok(())
    }
    pub async fn run_display(
        &self,
        mut response_recv: Receiver<DisplayResponseContainer>,
        status: Rc<Status>,
    ) -> Result<!, Error> {
        let mut state = DisplayState::Stopped(
            iter::repeat_n(heapless::String::from_str(" ").unwrap(), MAX_GLYPHS).collect(),
        );
        loop {
            match select(response_recv.recv(), self.handle_state(state.clone())).await {
                Either::First(None) => return Err(Error::UnexpectedEof),
                Either::First(Some(new)) => match new {
                    DisplayResponseContainer::DisplayResponse(response) => match response {
                        DisplayResponse::Start(_) => state = DisplayState::Running,
                        DisplayResponse::Stop(text) => state = DisplayState::Stopped(text),
                    },
                    DisplayResponseContainer::DeviceInfo(info) => {
                        status.set(StatusPriority::Info, "Connected!".to_string());
                        self.build(&info).unwrap_or_else(|e| error!("{:?}", e))
                    }
                },
                Either::Second(e) => return e,
            }
        }
    }
}
