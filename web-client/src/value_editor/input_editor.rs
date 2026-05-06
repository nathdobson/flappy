#![allow(dead_code)]
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::input_type::InputType;
use crate::utils::create_element;
use crate::value_editor::ValueEditor;
use empty_rc::EmptyRc;
use error_report::Report;
use protocol::setup::MAX_SETUP_MESSAGE_SIZE;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::rc::Rc;
use std::str::FromStr;
use web_sys::{Event, HtmlInputElement, Node};

pub struct InputEditor<T> {
    input: HtmlInputElement,
    from_str: Box<dyn Fn(&str) -> Result<T, Error>>,
    to_str: Box<dyn Fn(&T) -> String>,
    #[allow(dead_code)]
    listener: EventListener<'static>,
}

pub struct InputEditorBuilder<T> {
    from_str: Option<Box<dyn Fn(&str) -> Result<T, Error>>>,
    to_str: Option<Box<dyn Fn(&T) -> String>>,
    typ: InputType,
    class: &'static str,
    min: Option<T>,
    max: Option<T>,
    step: Option<T>,
    value: Option<T>,
}

impl<T: 'static> InputEditorBuilder<T> {
    pub fn new() -> Self {
        InputEditorBuilder {
            from_str: None,
            to_str: None,
            typ: InputType::Text,
            class: "text-editor",
            min: None,
            max: None,
            step: None,
            value: None,
        }
    }
    pub fn with_from_str_display(self) -> Self
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        self.with_from_str_to_str(|x| Ok(T::from_str(x)?), |x| x.to_string())
    }
    pub fn with_from_str_to_str(
        mut self,
        from_str: impl 'static + Fn(&str) -> Result<T, Error>,
        to_str: impl 'static + Fn(&T) -> String,
    ) -> Self {
        self.from_str = Some(Box::new(from_str));
        self.to_str = Some(Box::new(to_str));
        self
    }
    pub fn with_json_serde(self) -> Self
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        self.with_from_str_to_str(
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
    pub fn with_class(mut self, class: &'static str) -> Self {
        self.class = class;
        self
    }
    pub fn with_type(mut self, typ: InputType) -> Self {
        self.typ = typ;
        self
    }
    pub fn with_min(mut self, min: T) -> Self {
        self.min = Some(min);
        self
    }
    pub fn with_max(mut self, max: T) -> Self {
        self.max = Some(max);
        self
    }
    pub fn with_step(mut self, step: T) -> Self {
        self.step = Some(step);
        self
    }
    pub fn with_value(mut self, value: T) -> Self {
        self.value = Some(value);
        self
    }

    pub fn new_number() -> Self
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        Self::new()
            .with_type(InputType::Number)
            .with_from_str_display()
    }

    pub fn new_color() -> Self
    where
        T: 'static + FromStr + Display,
        Error: From<<T as FromStr>::Err>,
    {
        Self::new()
            .with_class("color-editor")
            .with_type(InputType::Color)
            .with_from_str_to_str(
                |x| Ok(T::from_str(x.trim_prefix("#"))?),
                |x| format!("#{}", x),
            )
    }

    #[track_caller]
    pub fn build(self) -> Result<Rc<InputEditor<T>>, Error> {
        let this = EmptyRc::new();
        let from_str = self.from_str.expect("from_str");
        let to_str = self.to_str.expect("to_str");
        let input = create_element::<"input">()?;
        input.set_type(self.typ.as_str());
        input.set_class_name(self.class);
        if let Some(min) = self.min {
            input.set_min(&to_str(&min));
        }
        if let Some(max) = self.max {
            input.set_max(&to_str(&max));
        }
        if let Some(step) = self.step {
            input.set_max(&to_str(&step));
        }
        if let Some(value) = self.value {
            input.set_value(&to_str(&value));
        }
        let listener = EventListener::new_weak(
            &input,
            EventType::Input,
            this.downgrade(),
            InputEditor::on_input,
        )?;
        let this = this.into_rc(InputEditor {
            input,
            from_str,
            to_str,
            listener,
        });
        this.set_custom_validity();
        Ok(this)
    }
}

impl<T: 'static> InputEditorBuilder<Option<T>> {
    pub fn with_optional(self) -> Self
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        self.with_from_str_to_str(from_str_option, to_str_option)
    }
}

impl<T: 'static> InputEditor<T> {
    fn set_custom_validity(&self) {
        if let Err(e) = (self.from_str)(&self.input.value()) {
            self.input
                .set_custom_validity(&format!("{}", Report::new(e)));
        } else {
            self.input.set_custom_validity("");
        }
    }

    fn on_input(self: Rc<Self>, _event: Event) {
        self.set_custom_validity();
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

impl<T: 'static + Default> ValueEditor<T> for InputEditor<T> {
    fn node(self: Rc<Self>) -> Node {
        self.input.clone().into()
    }

    fn set_value(self: Rc<Self>, value: &T) -> Result<(), Error> {
        self.input.set_custom_validity("");
        self.input.set_value(&(self.to_str)(value));
        Ok(())
    }

    fn get_value(self: Rc<Self>) -> Result<T, Error> {
        Ok((self.from_str)(&self.input.value())?)
    }
}
