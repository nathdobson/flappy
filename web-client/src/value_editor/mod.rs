pub mod struct_editor;
pub mod text_editor;
pub mod value_form;
pub mod select_editor;

use crate::error::Error;
use std::marker::PhantomData;
use web_sys::Node;

pub trait ValueEditor<T: 'static>: 'static {
    fn node(&self) -> &Node;
    fn set_value(&mut self, value: &T);
    fn get_value(&self) -> Result<T, Error>;
}
