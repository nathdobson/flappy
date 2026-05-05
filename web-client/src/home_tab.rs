use crate::built_info;
use crate::error::Error;
use crate::tabs::TabContent;
use crate::utils::AppendChild;
use crate::utils::create_element;
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
        li.set_text_content(Some(&format!(
            "Flappy Web Client version {}",
            built_info::GIT_VERSION.unwrap_or("<unknown>"),
        )));
        let li = ul.append_element::<"li">()?;
        let a = li.append_element::<"a">()?;
        a.append_text("GitHub Repository")?;
        a.set_href("https://github.com/nathdobson/flappy");
        Ok(HomeTab { node })
    }
}
