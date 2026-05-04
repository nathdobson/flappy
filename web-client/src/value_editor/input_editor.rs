use crate::bind_weak::bind_weak_fn1;
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::utils::create_element;
use crate::value_editor::ValueEditor;
use empty_rc::EmptyRc;
use error_report::Report;
use log::info;
use protocol::setup::MAX_SETUP_MESSAGE_SIZE;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::{RangeInclusive, RangeToInclusive};
use std::rc::Rc;
use std::str::FromStr;
use web_sys::{Event, HtmlInputElement, Node};

pub struct InputEditor<T> {
    input: HtmlInputElement,
    from_str: Box<dyn Fn(&str) -> Result<T, Error>>,
    to_str: Box<dyn Fn(&T) -> String>,
    listener: EventListener<'static>,
}

impl<T: 'static> InputEditor<T> {
    pub fn new() -> Result<Rc<Self>, Error>
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        Self::new_with(|x| Ok(T::from_str(x)?), |x| x.to_string())
    }

    pub fn new_with(
        from_str: impl 'static + Fn(&str) -> Result<T, Error>,
        to_str: impl 'static + Fn(&T) -> String,
    ) -> Result<Rc<Self>, Error> {
        let this = EmptyRc::new();
        let input = create_element::<"input">()?;
        input.set_type("text");
        input.set_class_name("text-editor");
        let listener = EventListener::new(
            &input,
            EventType::Change,
            bind_weak_fn1(this.downgrade(), Self::on_change),
        )?;
        Ok(this.into_rc(InputEditor {
            input,
            from_str: Box::new(from_str),
            to_str: Box::new(to_str),
            listener,
        }))
    }

    pub fn on_change(self: Rc<Self>, event: Event) -> bool {
        true
    }

    pub fn new_integer(min: Option<T>, max: Option<T>, step: Option<T>) -> Result<Rc<Self>, Error>
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        let result = Self::new()?;
        result.input.set_type("number");
        if let Some(min) = min {
            result.input.set_min(&min.to_string());
        }
        if let Some(max) = max {
            result.input.set_max(&max.to_string());
        }
        if let Some(step) = step {
            result.input.set_step(&step.to_string());
        }
        Ok(result)
    }

    pub fn new_json() -> Result<Rc<Self>, Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        Self::new_with(
            |str| {
                let mut buffer = vec![0; MAX_SETUP_MESSAGE_SIZE];
                let result = serde_json_core::from_str_escaped::<T>(str, &mut buffer)?;
                Ok(result.0)
            },
            |x| {
                serde_json_core::to_string::<T, MAX_SETUP_MESSAGE_SIZE>(&x)
                    .unwrap()
                    .to_string()
            },
        )
    }
}

fn from_str_option<T: FromStr>(x: &str) -> Result<Option<T>, Error>
where
    Error: From<T::Err>,
{
    if x.is_empty() {
        Ok(None)
    } else {
        Ok(Some(T::from_str(x)?))
    }
}
fn to_str_option<T: Display>(x: &Option<T>) -> String {
    if let Some(x) = x {
        x.to_string()
    } else {
        String::new()
    }
}

impl<T: 'static> InputEditor<Option<T>> {
    pub fn new_optional() -> Result<Rc<Self>, Error>
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        Self::new_with(from_str_option, to_str_option)
    }
    pub fn new_optional_integer(
        min: Option<T>,
        max: Option<T>,
        step: Option<T>,
    ) -> Result<Rc<Self>, Error>
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        let result = Self::new_optional()?;
        result.input.set_type("number");
        if let Some(min) = min {
            result.input.set_min(&min.to_string());
        }
        if let Some(max) = max {
            result.input.set_max(&max.to_string());
        }
        if let Some(step) = step {
            result.input.set_step(&step.to_string());
        }
        Ok(result)
    }
}

impl InputEditor<heapless::String<6>> {
    pub fn new_color() -> Result<Rc<Self>, Error> {
        let result = Self::new_with(
            |x| Ok(heapless::String::try_from(x.trim_prefix("#"))?),
            |x| format!("#{}", x),
        )?;
        result.input.set_type("color");
        Ok(result)
    }
}

impl<T: 'static + Default> ValueEditor<T> for InputEditor<T> {
    fn node(self: Rc<Self>) -> Node {
        self.input.clone().into()
    }

    fn set_value(self: Rc<Self>, value: &T) {
        self.input.set_value(&(self.to_str)(value));
    }

    fn get_value(self: Rc<Self>) -> Result<T, Error> {
        Ok((self.from_str)(&self.input.value())?)
    }
}
