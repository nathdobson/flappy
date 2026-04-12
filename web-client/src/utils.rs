use crate::error::Error;
use futures_util::future::FutureExt;
use log::info;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::select;
use tokio::sync::oneshot;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Bluetooth, Document, HtmlAnchorElement, HtmlButtonElement, HtmlDivElement, HtmlElement, HtmlFormElement, HtmlInputElement, HtmlLabelElement, HtmlLiElement, HtmlOListElement, HtmlUListElement, Node, Text, Window};

pub fn try_window() -> Result<Window, Error> {
    web_sys::window().ok_or(Error::CannotFindElement)
}

pub fn document() -> Result<Document, Error> {
    try_window()?.document().ok_or(Error::CannotFindElement)
}

pub fn bluetooth() -> Result<Bluetooth, Error> {
    try_window()?
        .navigator()
        .bluetooth()
        .ok_or(Error::BluetoothNotSupported)
}

pub fn get_element_by_id<T: JsCast>(id: &str) -> Result<T, Error> {
    let window = web_sys::window().ok_or(Error::CannotFindElement)?;
    let document = window.document().ok_or(Error::CannotFindElement)?;
    Ok(document
        .get_element_by_id(id)
        .ok_or(Error::CannotFindElement)?
        .dyn_into::<T>()
        .ok()
        .ok_or(Error::TypeError)?)
}

pub fn create_text_node(text: &str) -> Result<Text, Error> {
    Ok(document()?.create_text_node(text))
}

pub fn create_element<const TAG: &'static str>(
) -> Result<<Tag<TAG> as HasElementType>::ElementType, Error>
where
    Tag<TAG>: HasElementType,
{
    Ok(document()?
        .create_element(TAG)?
        .dyn_into()
        .ok()
        .ok_or(Error::TypeError)?)
}

pub trait AppendChild {
    fn append_element<const TAG: &'static str>(
        &self,
    ) -> Result<<Tag<TAG> as HasElementType>::ElementType, Error>
    where
        Tag<TAG>: HasElementType;
    fn append_text(&self, text: &str) -> Result<Text, Error>;
}

impl<T> AppendChild for T
where
    T: AsRef<Node>,
{
    fn append_element<const TAG: &'static str>(
        &self,
    ) -> Result<<Tag<TAG> as HasElementType>::ElementType, Error>
    where
        Tag<TAG>: HasElementType,
    {
        let child = create_element::<TAG>()?;
        self.as_ref().append_child(child.as_ref())?;
        Ok(child)
    }

    fn append_text(&self, text: &str) -> Result<Text, Error> {
        let text = create_text_node(text)?;
        self.as_ref().append_child(&text)?;
        Ok(text)
    }
}

pub struct Tag<const TAG: &'static str>;

pub trait HasElementType {
    type ElementType: JsCast + AsRef<Node>;
}

impl HasElementType for Tag<"div"> {
    type ElementType = HtmlDivElement;
}

impl HasElementType for Tag<"button"> {
    type ElementType = HtmlButtonElement;
}

impl HasElementType for Tag<"a"> {
    type ElementType = HtmlAnchorElement;
}

impl HasElementType for Tag<"li"> {
    type ElementType = HtmlLiElement;
}

impl HasElementType for Tag<"ul"> {
    type ElementType = HtmlUListElement;
}

impl HasElementType for Tag<"nav"> {
    type ElementType = HtmlElement;
}

impl HasElementType for Tag<"form"> {
    type ElementType = HtmlFormElement;
}

impl HasElementType for Tag<"input"> {
    type ElementType = HtmlInputElement;
}

impl HasElementType for Tag<"label"> {
    type ElementType = HtmlLabelElement;
}

impl HasElementType for Tag<"ol"> {
    type ElementType = HtmlOListElement;
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
    let (mut tx, rx) = oneshot::channel();
    spawn_local(async move {
        select!(
            _ = tx.closed() => {

            }
            result = AssertUnwindSafe(f).catch_unwind() => {
                tx.send(result.map_err(|e| e.into())).ok();
            }
        );
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
