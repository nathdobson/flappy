use crate::error::Error;
use crate::utils::create_element;
use crate::value_editor::ValueEditor;
use protocol::setup::MAX_SETUP_MESSAGE_SIZE;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;
use web_sys::{HtmlInputElement, Node};

pub struct TextEditor<T> {
    phantom: PhantomData<T>,
    input: HtmlInputElement,
    from_str: Box<dyn Fn(&str) -> Result<T, Error>>,
    to_str: Box<dyn Fn(&T) -> String>,
}

impl<T> TextEditor<T> {
    pub fn new() -> Result<Self, Error>
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        Self::new_with(|x| Ok(T::from_str(x)?), |x| x.to_string())
    }

    pub fn new_with(
        from_str: impl 'static + Fn(&str) -> Result<T, Error>,
        to_str: impl 'static + Fn(&T) -> String,
    ) -> Result<Self, Error> {
        let input = create_element::<"input">()?;
        input.set_type("text");
        input.set_class_name("text-editor");
        Ok(TextEditor {
            phantom: PhantomData,
            input,
            from_str: Box::new(from_str),
            to_str: Box::new(to_str),
        })
    }

    pub fn new_json() -> Result<Self, Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        Self::new_with(
            |str| {
                let mut buffer = vec![0; MAX_SETUP_MESSAGE_SIZE];
                let result = serde_json_core::from_str_escaped::<T>(str, &mut buffer)?;
                Ok(result.0)
            },
            |x| serde_json_core::to_string::<T, MAX_SETUP_MESSAGE_SIZE>(&x).unwrap().to_string(),
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

impl<T: 'static> TextEditor<Option<T>> {
    pub fn new_optional() -> Result<Self, Error>
    where
        T: FromStr + Display,
        Error: From<T::Err>,
    {
        Self::new_with(from_str_option, to_str_option)
    }
}

impl<T: 'static + Default> ValueEditor<T> for TextEditor<T> {
    fn node(&self) -> &Node {
        &self.input
    }

    fn set_value(&mut self, value: &T) {
        self.input.set_value(&(self.to_str)(value));
    }

    fn get_value(&self) -> Result<T, Error> {
        Ok((self.from_str)(&self.input.value())?)
    }
}
