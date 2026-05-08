use crate::error::Error;
use crate::event_listener::{EventListener, EventListenerSet, EventType};
use crate::utils::{AppendChild, create_element};
use crate::value_editor::ValueEditor;
use empty_rc::EmptyRc;
use error_report::Report;
use log::error;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use web_sys::{Event, HtmlDivElement, Node};

struct ListEntry<V, I> {
    editor: Rc<dyn ValueEditor<I>>,
    list: Weak<ListEditor<V, I>>,
    node: HtmlDivElement,

    #[allow(dead_code)]
    listener: EventListener<'static>,
}

pub struct ListEditor<V, I> {
    node: HtmlDivElement,
    entries_node: HtmlDivElement,
    entries: RefCell<Vec<Rc<ListEntry<V, I>>>>,
    phantom: PhantomData<V>,
    factory: Box<dyn Fn() -> Result<Rc<dyn ValueEditor<I>>, Error>>,
    #[allow(dead_code)]
    listeners: EventListenerSet<'static, Self>,
}

impl<V: 'static, I: 'static> ListEntry<V, I> {
    fn delete_entry(self: Rc<Self>, event: Event) {
        event.prevent_default();
        if let Err(e) = self.delete_entry_impl() {
            error!("Error removing entry: {}", Report::new(e));
        }
    }
    fn delete_entry_impl(self: &Rc<Self>) -> Result<(), Error> {
        if let Some(list) = self.list.upgrade() {
            let mut entries = list.entries.borrow_mut();
            let index = entries.iter().position(|x| Rc::ptr_eq(&x, &self)).unwrap();
            entries.remove(index);
            list.entries_node.remove_child(&self.node)?;
        }
        Ok(())
    }
}

impl<V: 'static, I: 'static> ListEditor<V, I> {
    pub fn new(
        factory: impl 'static + Fn() -> Result<Rc<dyn ValueEditor<I>>, Error>,
    ) -> Result<Rc<Self>, Error> {
        let this = EmptyRc::new();
        let mut listeners = EventListenerSet::new(this.downgrade());
        let node = create_element::<"div">()?;
        node.set_class_name("list-editor");
        let add = node.append_element::<"button">()?;
        add.set_type("button");
        add.set_text_content(Some("Append entry"));
        listeners.add(&add, EventType::Click, Self::add_entry)?;
        let entries_node = node.append_element::<"div">()?;
        entries_node.set_class_name("list-editor-entries");
        Ok(this.into_rc(ListEditor {
            node,
            entries_node,
            entries: RefCell::new(vec![]),
            phantom: PhantomData,
            factory: Box::new(factory),
            listeners,
        }))
    }

    fn add_entry(self: Rc<Self>, event: Event) {
        event.prevent_default();
        if let Err(err) = self.add_entry_impl() {
            error!("Error adding list entry: {:?}", Report::new(err));
        }
    }
    fn add_entry_impl(self: &Rc<Self>) -> Result<(), Error> {
        self.entries.borrow_mut().push(self.create_entry()?);
        Ok(())
    }

    fn create_entry(self: &Rc<Self>) -> Result<Rc<ListEntry<V, I>>, Error> {
        let entry = EmptyRc::new();
        let editor = (self.factory)()?;
        let node = self.entries_node.append_element::<"div">()?;
        node.set_class_name("list-editor-entry");
        node.append_child(&editor.clone().node())?;
        let remove = node.append_element::<"button">()?;
        remove.set_type("button");
        remove.set_text_content(Some("Remove"));
        let listener = EventListener::new_weak(
            &remove,
            EventType::Click,
            entry.downgrade(),
            ListEntry::delete_entry,
        )?;
        Ok(entry.into_rc(ListEntry {
            editor,
            list: Rc::downgrade(self),
            node,
            listener,
        }))
    }
}

impl<V: 'static, I: 'static> ValueEditor<V> for ListEditor<V, I>
where
    for<'a> &'a V: IntoIterator<Item = &'a I>,
    V: TryFrom<Vec<I>>,
    Error: From<<V as TryFrom<Vec<I>>>::Error>,
{
    fn node(self: Rc<Self>) -> Node {
        self.node.clone().into()
    }

    fn set_value(self: Rc<Self>, value: &V) -> Result<(), Error> {
        let mut entries = self.entries.borrow_mut();
        for entry in &*entries {
            self.entries_node.remove_child(&entry.node)?;
        }
        entries.clear();
        for new in value.into_iter() {
            let entry = self.create_entry()?;
            entry.editor.clone().set_value(new)?;
            entries.push(entry);
        }
        Ok(())
    }

    fn get_value(self: Rc<Self>) -> Result<V, Error> {
        Ok(self
            .entries
            .borrow()
            .iter()
            .map(|x| x.editor.clone().get_value())
            .try_collect::<Vec<I>>()?
            .try_into()?)
    }
}

fn _test<const N: usize, I: 'static>(
    editor: &ListEditor<heapless::Vec<I, N>, I>,
) -> &dyn ValueEditor<heapless::Vec<I, N>> {
    editor
}
