use crate::bind_weak::{ bind_weak_fnmut1};
use crate::error::Error;
use log::info;
use std::rc::{Rc, Weak};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast};
use web_sys::{Event, EventTarget};

pub struct EventListener<'a> {
    target: EventTarget,
    typ: EventType,
    closure: Closure<dyn 'a + FnMut(Event)>,
}

pub struct EventListenerSet<'a, T> {
    listeners: Vec<EventListener<'a>>,
    this: Weak<T>,
}

impl<'a, T: 'a> EventListenerSet<'a, T> {
    pub fn new(this: Weak<T>) -> Self {
        EventListenerSet {
            listeners: vec![],
            this,
        }
    }
    pub fn add(
        &mut self,
        target: &EventTarget,
        typ: EventType,
        callback: impl 'a + FnMut(Rc<T>, Event),
    ) -> Result<(), Error> {
        self.listeners
            .push(EventListener::new_weak(target, typ, self.this.clone(), callback)?);
        Ok(())
    }
}

#[allow(dead_code)]
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
        callback: impl 'a + FnMut(Event),
    ) -> Result<EventListener<'a>, Error> {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(Event)>);
        target.add_event_listener_with_callback(typ.name(), &closure.as_ref().unchecked_ref())?;
        Ok(EventListener {
            target: target.clone(),
            typ,
            closure,
        })
    }
    pub fn new_weak<T: 'a>(
        target: &EventTarget,
        typ: EventType,
        this: Weak<T>,
        callback: impl 'a + FnMut(Rc<T>, Event),
    ) -> Result<EventListener<'a>, Error> {
        Self::new(target, typ, bind_weak_fnmut1(this, callback))
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
