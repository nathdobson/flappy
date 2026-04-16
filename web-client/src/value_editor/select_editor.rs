use crate::error::Error;
use crate::utils::AppendChild;
use crate::utils::create_element;
use crate::value_editor::ValueEditor;
use std::fmt::Display;
use web_sys::{HtmlInputElement, HtmlSelectElement, Node};

pub struct SelectEditor<T> {
    values: Vec<T>,
    input: HtmlSelectElement,
}

impl<T: 'static + Eq + Display> SelectEditor<T> {
    pub fn new(values: Vec<T>) -> Result<Self, Error> {
        let input = create_element::<"select">()?;
        for (index, value) in values.iter().enumerate() {
            let option = input.append_element::<"option">()?;
            option.set_value(&format!("{}", index));
            option.set_text_content(Some(&format!("{}", value)));
        }
        Ok(SelectEditor { values, input })
    }
}

impl<T: 'static + Eq + Display + Clone> ValueEditor<T> for SelectEditor<T> {
    fn node(&self) -> &Node {
        &self.input
    }

    fn set_value(&mut self, value: &T) {
        self.input.set_value(&format!(
            "{}",
            self.values
                .iter()
                .position(|x| x == value)
                .expect("value not listed")
        ));
    }

    fn get_value(&self) -> Result<T, Error> {
        Ok(self.values[self.input.value().parse::<usize>()?].clone())
    }
}
