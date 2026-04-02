use crate::error::Error;
use crate::utils::{create_element, get_element_by_id};
use log::info;
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
    pub fn set(&self, priority: StatusPriority, value: String) {
        info!("Status[{:?}] = {}", priority, value);
        let mut old = self.priority.borrow_mut();
        if *old <= priority {
            *old = priority;
            self.element.set_text_content(Some(&value));
        }
    }
}
