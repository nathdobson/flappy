use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::query_params::{QueryParams, QueryParamsCell};
use crate::utils::{create_element, try_create_div, try_document};
use by_address::ByAddress;
use log::info;
use std::cell::{Cell, OnceCell};
use std::rc::Rc;
use web_sys::{Element, HtmlButtonElement, HtmlDivElement, Node};

struct Tab {
    content: Rc<dyn TabContent>,
    button: HtmlButtonElement,
    listener: OnceCell<EventListener<'static>>,
}

pub struct TabContainer {
    node: HtmlDivElement,
    tabs: Vec<Tab>,
    current: Cell<Option<usize>>,
    query_params: Rc<QueryParamsCell>,
}

pub trait TabContent: 'static {
    fn title(&self) -> &str;
    fn id(&self) -> &str;
    fn handle_visible(&self, visible: bool);
    fn node(&self) -> &HtmlDivElement;
}

impl TabContainer {
    pub fn new(
        content: Vec<Rc<dyn TabContent>>,
        query_params: Rc<QueryParamsCell>,
    ) -> Result<Rc<TabContainer>, Error> {
        let node = try_create_div()?;
        let header = try_create_div()?;
        header.set_class_name("tab-header");
        node.append_child(&header)?;
        let mut tabs = vec![];
        for (index, content) in content.into_iter().enumerate() {
            let content_node = content.node();
            content_node.style().set_property("display", "none")?;
            node.append_child(content_node)?;
            let button = create_element::<"button">()?;
            button.set_inner_text(content.title());
            button.set_class_name("tab-button tab-button-inactive");
            header.append_child(&button)?;
            tabs.push(Tab {
                content,
                button,
                listener: OnceCell::new(),
            });
        }
        let container = Rc::new(TabContainer {
            node: node.clone(),
            tabs,
            current: Cell::new(None),
            query_params,
        });
        for (index, tab) in container.tabs.iter().enumerate() {
            let listener = EventListener::new(tab.button.clone().into(), EventType::Click, {
                let container = Rc::downgrade(&container);
                move |_| {
                    if let Some(container) = container.upgrade() {
                        container.select(index).ok();
                    }
                    true
                }
            })?;
            tab.listener.set(listener).ok().unwrap();
        }
        if !container.tabs.is_empty() {
            let start = container
                .tabs
                .iter()
                .position(|t| t.content.id() == container.query_params.borrow().tab)
                .unwrap_or(0);
            container.select(start)?;
        }
        Ok(container)
    }
    fn select(&self, index: usize) -> Result<(), Error> {
        if let Some(old) = self.current.take() {
            self.tabs[old]
                .content
                .node()
                .style()
                .set_property("display", "none")
                .ok();
            self.tabs[old]
                .button
                .set_class_name("tab-button tab-button-inactive");
        }

        self.current.set(Some(index));
        self.tabs[index]
            .content
            .node()
            .style()
            .remove_property("display")
            .ok();
        self.tabs[index]
            .button
            .set_class_name("tab-button tab-button-active");
        if self.query_params.borrow().tab != self.tabs[index].content.id() {
            self.query_params.modify(|qp| {
                info!("{:?} != {:?}", qp.tab, self.tabs[index].content.id());
                qp.tab = self.tabs[index].content.id().to_string();
            })?;
        }
        Ok(())
    }
    pub fn node(&self) -> &HtmlDivElement {
        &self.node
    }
}
