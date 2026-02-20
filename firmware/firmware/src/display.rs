use crate::application::DisplayResponseContainer;
use crate::make_static;
use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use heapless::{String, Vec};
use log::{error, info};
use protocol::display::DisplayResponse;
use protocol::display::MAX_GLYPH_BYTES;
use protocol::display::MAX_GLYPHS;
use protocol::setup::{DisplaySettings, FLAP_COUNT};

const MODULE: &'static str = "[DISPL]";
pub struct DisplayModule {
    #[cfg(feature = "display")]
    controller: &'static crate::controller::ControllerModule,
    display_response: &'static Channel<NoopRawMutex, DisplayResponseContainer, 1>,
    settings: RefCell<DisplaySettings>,
}

impl DisplayModule {
    pub fn new(
        #[cfg(feature = "display")] controller: &'static crate::controller::ControllerModule,
        display_response: &'static Channel<NoopRawMutex, DisplayResponseContainer, 1>,
    ) -> &'static Self {
        make_static!(
            DisplayModule,
            DisplayModule {
                #[cfg(feature = "display")]
                controller,
                display_response,
                settings: RefCell::new(DisplaySettings::default()),
            }
        )
    }
    pub fn set_settings(&self, settings: DisplaySettings) {
        self.settings.replace(settings);
    }
    fn render(
        &'static self,
        msg: &str,
    ) -> (
        Vec<usize, MAX_GLYPHS>,
        Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS>,
    ) {
        let glyphs_owned = self.settings.borrow().glyphs.clone();
        let mut glyphs: Vec<&str, FLAP_COUNT> = Vec::new();
        for x in &glyphs_owned {
            glyphs.push(&*x).unwrap();
        }
        let mut renderer = glyph_render::Renderer::<MAX_GLYPHS>::new(&glyphs);
        if let Err(e) = renderer.append(&msg) {
            error!("{MODULE} error when rendering message: {:?}", e);
        }
        let message = renderer.finish();
        let mut glyph_strs: Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS> = Vec::new();
        for i in message.iter() {
            glyph_strs
                .push(glyphs[*i].try_into().unwrap_or(" ".try_into().unwrap()))
                .unwrap();
        }
        (message, glyph_strs)
    }
    pub async fn display_once(&'static self, msg: &str) {
        let (message, message_strs) = self.render(&msg);
        self.display_response
            .send(DisplayResponseContainer::DisplayResponse(
                DisplayResponse::Start(message_strs.clone()),
            ))
            .await;
        #[cfg(not(feature = "display"))]
        Timer::after_millis(1000).await;
        #[cfg(feature = "display")]
        {
            info!("{MODULE} Displaying {}", msg);
            // display.set_settings(self.state.borrow().display.clone());
            if let Err(e) = self.controller.run(&message).await {
                error!("{MODULE} error when displaying message: {:?}", e);
            }
        }
        self.display_response
            .send(DisplayResponseContainer::DisplayResponse(
                DisplayResponse::Stop(message_strs),
            ))
            .await;
    }
}
