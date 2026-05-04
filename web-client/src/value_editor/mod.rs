pub mod bool_editor;
pub mod input_editor;
pub mod select_editor;
pub mod struct_editor;
pub mod value_form;

use crate::error::Error;
use std::marker::PhantomData;
use std::rc::Rc;
use web_sys::Node;

pub trait ValueEditor<T: 'static>: 'static {
    fn node(self: Rc<Self>) -> Node;
    fn set_value(self: Rc<Self>, value: &T);
    fn get_value(self: Rc<Self>) -> Result<T, Error>;
}
