use crate::error::Error;
use futures_util::future::FutureExt;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, HtmlButtonElement, HtmlDivElement, HtmlElement, HtmlFormElement, Text, Window};

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

pub fn try_create_text_node(text: &str) -> Result<Text, Error> {
    Ok(try_document()?.create_text_node(text))
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

impl HasElementType for Tag<"button"> {
    type ElementType = HtmlButtonElement;
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

pub struct JoinHandle<T>(oneshot::Receiver<Result<T, Error>>);

pub fn spawn_local_joinable<F: 'static + Future>(f: F) -> JoinHandle<F::Output>
where
    F::Output: 'static,
{
    let (tx, rx) = oneshot::channel();
    spawn_local(async move {
        tx.send(
            AssertUnwindSafe(f)
                .catch_unwind()
                .await
                .map_err(|e| e.into()),
        )
        .ok();
    });
    JoinHandle(rx)
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.get_mut().0).poll(cx) {
            Poll::Ready(x) => Poll::Ready(Ok(x??)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> JoinHandle<Result<T, Error>> {
    pub async fn try_join(mut self) -> Result<T, Error> {
        Ok(self.await??)
    }
}
