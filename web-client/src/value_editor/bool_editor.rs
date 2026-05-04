use crate::error::Error;
use crate::utils::create_element;
use crate::value_editor::ValueEditor;
use std::rc::Rc;
use web_sys::{HtmlInputElement, Node};

pub struct BoolEditor {
    input: HtmlInputElement,
}

impl BoolEditor {
    pub fn new() -> Result<Rc<Self>, Error> {
        let input = create_element::<"input">()?;
        input.set_type("checkbox");
        input.set_class_name("bool-editor");
        Ok(Rc::new(BoolEditor { input }))
    }
}

impl ValueEditor<bool> for BoolEditor {
    fn node(self: Rc<Self>) -> Node {
        self.input.clone().into()
    }

    fn set_value(self: Rc<Self>, value: &bool) {
        self.input.set_checked(*value)
    }

    fn get_value(self: Rc<Self>) -> Result<bool, Error> {
        Ok(self.input.checked())
    }
}
