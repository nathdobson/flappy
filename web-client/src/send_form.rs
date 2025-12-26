use wasm_bindgen::JsCast;
use crate::error::Error;
use crate::utils::try_get_element_by_id;
use wasm_bindgen::closure::Closure;
use web_sys::{HtmlFormElement, HtmlInputElement};

pub struct SendForm {
    form: HtmlFormElement,
    submit: HtmlInputElement,
    input: HtmlInputElement,
}

impl SendForm {
    pub fn new() -> Result<Self, Error> {
        let form: HtmlFormElement = try_get_element_by_id("form")?;
        let input: HtmlInputElement = try_get_element_by_id("content")?;
        let submit: HtmlInputElement = try_get_element_by_id("submit")?;
        Ok(SendForm {
            form,
            submit,
            input,
        })
    }
    pub fn set_on_submit(&self, mut on_submit: impl FnMut(String)) {
        let closure = Closure::wrap(Box::new(move || {
            on_submit(self.input.value());
            false
        }) as Box<dyn FnMut() -> bool>);
        self.submit.set_disabled(false);
        self.form
            .set_onsubmit(Some(&closure.as_ref().unchecked_ref()));
        closure.forget();
    }
}
