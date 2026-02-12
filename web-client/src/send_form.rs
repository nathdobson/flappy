use crate::error::Error;
use crate::utils::try_get_element_by_id;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{HtmlFormElement, HtmlInputElement, HtmlTextAreaElement};

pub struct SendForm {
    form: HtmlFormElement,
    submit: HtmlInputElement,
    input: HtmlInputElement,
    form_src: HtmlFormElement,
    input_src: HtmlTextAreaElement,
    submit_src: HtmlInputElement,
}

impl SendForm {
    pub fn new() -> Result<Self, Error> {
        let form: HtmlFormElement = try_get_element_by_id("form")?;
        let input: HtmlInputElement = try_get_element_by_id("content")?;
        let submit: HtmlInputElement = try_get_element_by_id("submit")?;

        let form_src: HtmlFormElement = try_get_element_by_id("form-src")?;
        let input_src: HtmlTextAreaElement = try_get_element_by_id("content-src")?;
        let submit_src: HtmlInputElement = try_get_element_by_id("submit-src")?;
        Ok(SendForm {
            form,
            submit,
            input,

            form_src,
            input_src,
            submit_src
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
    pub fn set_on_submit_src(&self, mut on_submit: impl FnMut(String)) {
        let closure = Closure::wrap(Box::new(move || {
            on_submit(self.input_src.value());
            false
        }) as Box<dyn FnMut() -> bool>);
        self.submit_src.set_disabled(false);
        self.form_src
            .set_onsubmit(Some(&closure.as_ref().unchecked_ref()));
        closure.forget();
    }
}
