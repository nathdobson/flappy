pub mod bool_editor;
pub mod input_editor;
pub mod list_editor;
pub mod select_editor;
pub mod struct_editor;
pub mod value_form;
pub mod calibration_editor;

use crate::error::Error;
use std::rc::Rc;
use web_sys::Node;

pub trait ValueEditor<T: 'static>: 'static {
    fn node(self: Rc<Self>) -> Node;
    fn set_value(self: Rc<Self>, value: &T) -> Result<(), Error>;
    fn get_value(self: Rc<Self>) -> Result<T, Error>;
}
