use crate::error::Error;
use std::marker::PhantomData;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlDivElement, HtmlElement, HtmlFormElement, Window};

pub fn try_window() -> Result<Window, Error> {
    web_sys::window().ok_or(Error::NoneError)
}

pub fn try_document() -> Result<Document, Error> {
    try_window()?.document().ok_or(Error::NoneError)
}

pub fn try_get_element_by_id<T: JsCast>(id: &str) -> Result<T, Error> {
    let window = web_sys::window().ok_or(Error::NoneError)?;
    let document = window.document().ok_or(Error::NoneError)?;
    Ok(document
        .get_element_by_id(id)
        .ok_or(Error::NoneError)?
        .dyn_into::<T>()
        .ok()
        .ok_or(Error::TypeError)?)
}

pub fn try_create_div() -> Result<HtmlDivElement, Error> {
    Ok(try_document()?
        .create_element("div")?
        .dyn_into()
        .ok()
        .ok_or(Error::TypeError)?)
}

pub fn create_element<const TAG: &'static str>(
) -> Result<<Tag<TAG> as HasElementType>::ElementType, Error>
where
    Tag<TAG>: HasElementType,
{
    Ok(try_document()?
        .create_element(TAG)?
        .dyn_into()
        .ok()
        .ok_or(Error::TypeError)?)
}

pub struct Tag<const TAG: &'static str>;

pub trait HasElementType {
    type ElementType: JsCast;
}

impl HasElementType for Tag<"div"> {
    type ElementType = HtmlDivElement;
}

pub async fn sleep(millis: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis)
            .unwrap();
    };
    let p = js_sys::Promise::new(&mut cb);
    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}
