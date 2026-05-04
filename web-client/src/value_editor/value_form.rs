use crate::dyn_async_fn::DynAsyncFn;
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::utils::create_element;
use crate::utils::AppendChild;
use crate::value_editor::ValueEditor;
use empty_rc::EmptyRc;
use error_report::Report;
use js_sys::futures::spawn_local;
use log::warn;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{HtmlDivElement, HtmlFormElement, HtmlInputElement};

pub struct ValueForm<T> {
    form: HtmlFormElement,
    value: Rc<dyn ValueEditor<T>>,
    submit: HtmlInputElement,
    submit_status: HtmlDivElement,
    on_submit: RefCell<Option<Rc<dyn DynAsyncFn(T) -> Result<(), Error>>>>,
    #[allow(dead_code)]
    event_listener: EventListener<'static>,
}

impl<T: 'static> ValueForm<T> {
    pub fn new(value: Rc<dyn ValueEditor<T>>) -> Result<Rc<Self>, Error> {
        let this = EmptyRc::<Self>::new();
        let form = create_element::<"form">()?;
        form.set_class_name("editor-form");
        form.append_child(&value.clone().node())?;
        let submit = form.append_element::<"input">()?;
        submit.set_type("submit");
        submit.set_value("Save");
        let submit_status = form.append_element::<"div">()?;
        let event_listener = EventListener::new(&form, EventType::Submit, {
            let this = this.downgrade();
            move |e| {
                e.prevent_default();
                if let Some(this) = this.upgrade() {
                    this.submit_status.set_text_content(Some("Submitting..."));
                    if let Err(e) = this.clone().do_submit() {
                        this.submit_status
                            .set_text_content(Some(&format!("Submit failure: {}", Report::new(e))));
                    } else {
                        this.submit_status.set_text_content(Some("Submitted"));
                    }
                }
            }
        })?;
        let this = this.into_rc(ValueForm {
            form,
            value,
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
        let value = self.value.clone().get_value()?;
        let on_submit = self.on_submit.borrow().clone();
        if let Some(on_submit) = on_submit {
            spawn_local(async move {
                let on_submit = on_submit.call(value);
                if let Err(e) = on_submit.await {
                    self.submit_status
                        .set_text_content(Some(&format!("Submit failure: {}", Report::new(&e))));
                    warn!("Submit failure: {}", Report::new(&e));
                }
            });
        }
        Ok(())
    }
    pub fn node(&self) -> &HtmlFormElement {
        &self.form
    }
    pub fn set_value(&self, value: &T) {
        self.value.clone().set_value(value);
    }
    pub fn set_on_submit<F>(&self, on_submit: F)
    where
        F: 'static + AsyncFn(T) -> Result<(), Error>,
    {
        *self.on_submit.borrow_mut() = Some(Rc::new(on_submit));
    }
}
