use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::status::StatusPriority;
use crate::utils::create_element;
use crate::utils::AppendChild;
use crate::value_editor::ValueEditor;
use empty_rc::EmptyRc;
use futures_util::future::{BoxFuture, LocalBoxFuture};
use js_sys::futures::spawn_local;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use web_sys::{HtmlDivElement, HtmlFormElement, HtmlInputElement, Text};

pub struct ValueForm<T> {
    form: HtmlFormElement,
    value: RefCell<Box<dyn ValueEditor<T>>>,
    submit: HtmlInputElement,
    submit_status: HtmlDivElement,
    event_listener: EventListener<'static>,
    on_submit: RefCell<Option<Rc<dyn Fn(T) -> LocalBoxFuture<'static, Result<(), Error>>>>>,
}

impl<T: 'static> ValueForm<T> {
    pub fn new(value: impl ValueEditor<T>) -> Result<Rc<Self>, Error> {
        let this = EmptyRc::<Self>::new();
        let form = create_element::<"form">()?;
        form.set_class_name("editor-form");
        let value = Box::new(value);
        form.append_child(value.node())?;
        let submit = form.append_element::<"input">()?;
        submit.set_type("submit");
        submit.set_value("Save");
        let submit_status = form.append_element::<"div">()?;
        let event_listener = EventListener::new(&form, EventType::Submit, {
            let this = this.downgrade();
            move |e| {
                e.prevent_default();
                if let Some(this) = this.upgrade() {
                    if let Err(e) = this.clone().do_submit() {
                        this.submit_status
                            .set_text_content(Some(&format!("Submit failure: {}", e)));
                    }
                }
                false
            }
        })?;
        let this = this.into_rc(ValueForm {
            form,
            value: RefCell::new(value),
            submit,
            submit_status,
            event_listener,
            on_submit: RefCell::new(None),
        });

        Ok(this)
    }
    pub fn set_submit_name(&self, name: &str) {
        self.submit.set_value(name);
    }
    fn do_submit(self: Rc<Self>) -> Result<(), Error> {
        let value = self.value.borrow().get_value()?;
        let on_submit = self.on_submit.borrow().clone();
        if let Some(on_submit) = on_submit {
            let on_submit = on_submit(value);
            spawn_local(async move {
                if let Err(e) = on_submit.await {
                    self.submit_status.set_text_content(Some(&format!("{}", e)));
                }
            });
        }
        Ok(())
    }
    pub fn node(&self) -> &HtmlFormElement {
        &self.form
    }
    pub fn set_value(&self, value: &T) {
        self.value.borrow_mut().set_value(value);
    }
    pub fn set_on_submit(
        &self,
        on_submit: impl 'static + Fn(T) -> LocalBoxFuture<'static, Result<(), Error>>,
    ) {
        *self.on_submit.borrow_mut() = Some(Rc::new(on_submit));
    }
}
