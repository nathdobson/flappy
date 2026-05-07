use crate::error::Error;
use crate::utils::create_element;
use error_report::Report;
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{HtmlDivElement, HtmlElement};

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum StatusPriority {
    Info,
    Error,
}

pub struct Status {
    element: HtmlDivElement,
    priority: RefCell<StatusPriority>,
}

impl Status {
    pub fn new() -> Result<Rc<Status>, Error> {
        let element = create_element::<"div">()?;
        Ok(Rc::new(Self {
            element,
            priority: RefCell::new(StatusPriority::Info),
        }))
    }
    pub fn node(&self) -> &HtmlElement {
        &self.element
    }
    pub fn reset(&self) {
        *self.priority.borrow_mut() = StatusPriority::Info;
    }
    pub fn set(&self, priority: StatusPriority, value: String) {
        info!("Status[{:?}] = {}", priority, value);
        let mut old = self.priority.borrow_mut();
        if *old <= priority {
            *old = priority;
            self.element.set_text_content(Some(&value));
        }
    }
    pub fn set_error(
        &self,
        priority: StatusPriority,
        prefix: &str,
        error: &dyn core::error::Error,
    ) {
        error!("{:?}", error);
        self.set(priority, format!("{}: {}", prefix, Report::new(error)));
    }
}
