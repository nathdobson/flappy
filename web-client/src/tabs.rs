use crate::error::Error;
use crate::query_params::QueryParamsCell;
use crate::utils::create_element;
use std::cell::Cell;
use std::rc::Rc;
use web_sys::{HtmlAnchorElement, HtmlDivElement, HtmlElement, HtmlLiElement};

struct Tab {
    content: Rc<dyn TabContent>,
    anchor: HtmlAnchorElement,
    li: HtmlLiElement,
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
    fn node(&self) -> &HtmlElement;
}

impl TabContainer {
    pub fn new(
        content: Vec<Rc<dyn TabContent>>,
        default: usize,
        query_params: Rc<QueryParamsCell>,
    ) -> Result<Rc<TabContainer>, Error> {
        let node = create_element::<"div">()?;
        node.set_class_name("container");
        let header = create_element::<"nav">()?;
        header.set_class_name("tab-header");
        node.append_child(&header)?;
        let mut tabs = vec![];
        for content in content.into_iter() {
            let content_node = content.node();
            content_node.style().set_property("display", "none")?;
            node.append_child(content_node)?;
            let anchor = create_element::<"a">()?;
            anchor.set_href(&format!("?tab={}", content.id()));
            anchor.set_inner_text(content.title());
            let li = create_element::<"li">()?;
            li.append_child(&anchor)?;
            header.append_child(&li)?;
            tabs.push(Tab {
                content,
                anchor,
                li,
            });
        }
        let container = Rc::new(TabContainer {
            node: node.clone(),
            tabs,
            current: Cell::new(None),
            query_params,
        });
        if !container.tabs.is_empty() {
            let index = container
                .tabs
                .iter()
                .position(|t| t.content.id() == container.query_params.borrow().tab)
                .unwrap_or(default);
            container.current.set(Some(index));
            container.tabs[index].anchor.set_class_name("active");
            container.tabs[index]
                .content
                .node()
                .style()
                .remove_property("display")?;
        }
        Ok(container)
    }
    pub fn node(&self) -> &HtmlDivElement {
        &self.node
    }
}
