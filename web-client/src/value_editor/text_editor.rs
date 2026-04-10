use crate::error::Error;
use crate::utils::create_element;
use crate::value_editor::ValueEditor;
use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;
use web_sys::{HtmlInputElement, Node};

pub struct TextEditor<T> {
    phantom: PhantomData<T>,
    input: HtmlInputElement,
}

impl<T> TextEditor<T> {
    pub fn new() -> Result<Self, Error> {
        let input = create_element::<"input">()?;
        input.set_type("text");
        input.set_class_name("text-editor");
        Ok(TextEditor {
            phantom: PhantomData,
            input,
        })
    }
}

impl<T: 'static + Default + FromStr + Display> ValueEditor<T> for TextEditor<T>
where
    Error: From<T::Err>,
{
    fn node(&self) -> &Node {
        &self.input
    }

    fn set_value(&mut self, value: &T) {
        self.input.set_value(&value.to_string());
    }

    fn get_value(&self) -> Result<T, Error> {
        Ok(T::from_str(&self.input.value())?)
    }
}
