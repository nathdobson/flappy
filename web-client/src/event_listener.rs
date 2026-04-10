use crate::error::Error;
use log::info;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, EventTarget};

pub struct EventListener<'a> {
    target: EventTarget,
    typ: EventType,
    closure: Closure<dyn 'a + FnMut(Event) -> bool>,
}

pub enum EventType {
    Submit,
    Click,
    Change,
    Input,
    CharacteristicValueChanged,
}

impl<'a> EventListener<'a> {
    pub fn new(
        target: &EventTarget,
        typ: EventType,
        callback: impl 'a + FnMut(Event) -> bool,
    ) -> Result<EventListener<'a>, Error> {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(Event) -> bool>);
        target.add_event_listener_with_callback(typ.name(), &closure.as_ref().unchecked_ref())?;
        Ok(EventListener {
            target: target.clone(),
            typ,
            closure,
        })
    }
}

impl EventType {
    pub fn name(&self) -> &'static str {
        match self {
            EventType::Submit => "submit",
            EventType::Click => "click",
            EventType::Change => "change",
            EventType::Input => "input",
            EventType::CharacteristicValueChanged => "characteristicvaluechanged",
        }
    }
}

impl<'a> Drop for EventListener<'a> {
    fn drop(&mut self) {
        info!("Removing event listener");
        let _ = self.target.remove_event_listener_with_callback(
            self.typ.name(),
            self.closure.as_ref().unchecked_ref(),
        );
    }
}
