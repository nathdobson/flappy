use crate::error::Error;
use crate::utils::try_window;
use serde::{Deserialize, Serialize};
use std::cell::{Ref, RefCell, RefMut};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryParams {
    #[serde(default)]
    pub tab: String,
    pub ws_url: String,
    pub username: String,
    pub password: String,
    pub topic: String,
    #[serde(default)]
    pub spindle: bool,
}

pub struct QueryParamsCell {
    inner: RefCell<QueryParams>,
}

impl QueryParamsCell {
    pub fn new() -> Result<Self, Error> {
        let search = try_window()?.location().search()?;
        let search = search.strip_prefix("?").unwrap_or(&search);
        let params = serde_qs::from_str(&search)?;
        Ok(QueryParamsCell {
            inner: RefCell::new(params),
        })
    }
    pub fn borrow(&self) -> Ref<'_, QueryParams> {
        self.inner.borrow()
    }
    pub fn modify(&self, f: impl FnOnce(&mut QueryParams)) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();
        f(&mut *inner);
        let search = serde_qs::to_string(&*inner)?;
        try_window()?
            .location()
            .set_search(&format!("?{}", search))?;
        Ok(())
    }
}
