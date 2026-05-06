use crate::error::Error;
use crate::event_listener::{EventListenerSet, EventType};
use crate::utils::{AppendChild, create_element};
use crate::value_editor::ValueEditor;
use crate::value_editor::input_editor::{InputEditor, InputEditorBuilder};
use empty_rc::EmptyRc;
use error_report::Report;
use log::error;
use std::rc::Rc;
use web_sys::{Event, HtmlDivElement, Node};

pub struct CalibrationEditor {
    node: HtmlDivElement,
    input: Rc<InputEditor<usize>>,
    modulus: usize,
    #[allow(dead_code)]
    listeners: EventListenerSet<'static, Self>,
}

impl CalibrationEditor {
    pub fn new(modulus: usize) -> Result<Rc<Self>, Error> {
        let this = EmptyRc::<Self>::new();
        let mut listeners = EventListenerSet::new(this.downgrade());
        let input = InputEditorBuilder::new_number()
            .with_min(0)
            .with_max(modulus - 1)
            .with_value(0)
            .build()?;
        let node = create_element::<"div">()?;
        node.set_class_name("calibration-editor");
        node.append_child(&input.clone().node())?;
        for adjust in [-10, 10] {
            let button = node.append_element::<"button">()?;
            button.set_type("button");
            let label = if adjust < 0 {
                format!("{}", adjust)
            } else {
                format!("+{}", adjust)
            };
            button.set_text_content(Some(&label));
            listeners.add(&button, EventType::Click, move |this, event| {
                if let Err(e) = this.adjust(event, adjust) {
                    error!("Error adjusting calibration state: {}", Report::new(e));
                }
            })?;
        }
        Ok(this.into_rc(CalibrationEditor {
            node,
            input,
            listeners,
            modulus,
        }))
    }
    fn adjust(self: Rc<Self>, event: Event, adjust: isize) -> Result<(), Error> {
        event.prevent_default();
        let value = self.input.clone().get_value()? as isize;
        let value = (value + adjust).rem_euclid(self.modulus as isize);
        self.input.clone().set_value(&(value as usize))?;
        Ok(())
    }
}

impl ValueEditor<usize> for CalibrationEditor {
    fn node(self: Rc<Self>) -> Node {
        self.node.clone().into()
    }

    fn set_value(self: Rc<Self>, value: &usize) -> Result<(), Error> {
        self.input.clone().set_value(value)
    }

    fn get_value(self: Rc<Self>) -> Result<usize, Error> {
        self.input.clone().get_value()
    }
}
