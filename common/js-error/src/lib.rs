use std::error::Error;
use std::fmt::{Display, Formatter};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::DomException;

#[derive(Debug, Clone)]
pub struct JsValueError(JsValue);

impl Error for JsValueError {}

impl Display for JsValueError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Ok(e) = self.0.clone().dyn_into::<DomException>() {
            write!(f, "{} ({})", e.message(), e.name())?;
        } else {
            write!(f, "JsError: {:?}", self.0)?;
        }
        Ok(())
    }
}

impl From<JsValue> for JsValueError {
    fn from(err: JsValue) -> Self {
        Self(err)
    }
}
