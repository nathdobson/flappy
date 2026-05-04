use crate::error::Error;
use crate::tabs::TabContent;
use crate::utils::create_element;
use crate::utils::AppendChild;
use web_sys::{HtmlDivElement, HtmlElement};

pub struct HomeTab {
    node: HtmlDivElement,
}

impl TabContent for HomeTab {
    fn title(&self) -> &str {
        "Home"
    }

    fn id(&self) -> &str {
        "home"
    }

    fn node(&self) -> &HtmlElement {
        &self.node
    }
}

impl HomeTab {
    pub fn new() -> Result<Self, Error> {
        let node = create_element::<"div">()?;
        let ul = node.append_element::<"ul">()?;
        let li = ul.append_element::<"li">()?;
        let a = li.append_element::<"a">()?;
        a.append_text("GitHub Repository")?;
        a.set_href("https://github.com/nathdobson/flappy");
        Ok(HomeTab { node })
    }
}
