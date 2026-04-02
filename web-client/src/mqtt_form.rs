use std::rc::Rc;
use crate::error::Error;
use crate::query_params::{QueryParams, QueryParamsCell};
use crate::utils::get_element_by_id;
use web_sys::{HtmlFormElement, HtmlInputElement};

pub struct MqttForm {}

impl MqttForm {
    pub fn new(params: &Rc<QueryParamsCell>) -> Result<Self, Error> {
        let params=params.borrow();
        get_element_by_id::<HtmlInputElement>("ws_url")?.set_value(&params.ws_url);
        get_element_by_id::<HtmlInputElement>("username")?.set_value(&params.username);
        get_element_by_id::<HtmlInputElement>("password")?.set_value(&params.password);
        get_element_by_id::<HtmlInputElement>("topic")?.set_value(&params.topic);
        get_element_by_id::<HtmlInputElement>("settings-submit")?.set_disabled(false);
        Ok(Self {})
    }
}
