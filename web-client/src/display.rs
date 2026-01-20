use crate::error::Error;
use crate::utils::{create_element, sleep, try_get_element_by_id};
use log::info;
use protocol::display::{
    DisplayRequest, DisplayResponse, DISPLAY_REQUEST_CAPACITY, MAX_GLYPHS, MAX_GLYPH_BYTES,
};
use protocol::setup::DeviceInfo;
use std::future::pending;
use web_sys::{HtmlDivElement, HtmlElement};

pub struct Display {
    inners: Vec<HtmlDivElement>,
    dots: Vec<char>,
}

#[derive(Clone)]
pub enum DisplayState {
    Running,
    Stopped(heapless::Vec<heapless::String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
}

impl Display {
    pub fn new() -> Result<Self, Error> {
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
            inners: vec![],
            dots,
        })
    }
    pub async fn handle_state(&self, resp: DisplayState) -> Result<!, Error> {
        match resp {
            DisplayState::Running => {
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
            DisplayState::Stopped(text) => {
                for (index, inner) in self.inners.iter().enumerate() {
                    inner.set_text_content(Some(text.get(index).map_or(" ", |x| &**x)));
                }
            }
        }
        pending().await
    }
    pub fn build(&mut self, info: &DeviceInfo) -> Result<(), Error> {
        info!("DeviceInfo = {:?}", info);
        let display: HtmlElement = try_get_element_by_id("display")?;
        display
            .style()
            .set_property("color", &format!("#{}", info.foreground))?;
        for inner in &self.inners {
            display.remove_child(inner)?;
        }
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
