use crate::error::Error;
use crate::utils::{create_element, AppendChild};
use crate::value_editor::ValueEditor;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use web_sys::{HtmlDivElement, Node};

trait FieldEditor<S: 'static> {
    fn node(&self) -> Node;
    fn set_value(&self, value: &S);
    fn get_value(&self, value: &mut S) -> Result<(), Error>;
}

struct ValueFieldEditor<S, F> {
    inner: Rc<dyn ValueEditor<F>>,
    field: Field<S, F>,
}

impl<S: 'static, F: 'static> FieldEditor<S> for ValueFieldEditor<S, F> {
    fn node(&self) -> Node {
        self.inner.clone().node()
    }

    fn set_value(&self, value: &S) {
        self.inner.clone().set_value((self.field.get)(value));
    }

    fn get_value(&self, value: &mut S) -> Result<(), Error> {
        *(self.field.get_mut)(value) = self.inner.clone().get_value()?;
        Ok(())
    }
}

pub struct StructEditor<S> {
    phantom: PhantomData<S>,
    div: HtmlDivElement,
    fields: RefCell<Vec<Box<dyn FieldEditor<S>>>>,
}

pub struct Field<S, F> {
    pub name: &'static str,
    pub get: Box<dyn Fn(&S) -> &F>,
    pub get_mut: Box<dyn Fn(&mut S) -> &mut F>,
}

#[macro_export]
macro_rules! field {
    ($label:literal, $field:ident) => {
        $crate::value_editor::struct_editor::Field {
            name: $label,
            get: Box::new(|x| &x.$field),
            get_mut: Box::new(|x| &mut x.$field),
        }
    };
}

impl<S: 'static> StructEditor<S> {
    pub fn new() -> Result<Rc<Self>, Error> {
        let div = create_element::<"div">()?;
        div.set_class_name("struct-editor");
        Ok(Rc::new(StructEditor {
            div,
            phantom: PhantomData,
            fields: RefCell::new(vec![]),
        }))
    }
    pub fn add<F: 'static>(
        &self,
        field: Field<S, F>,
        value: Rc<dyn ValueEditor<F>>,
    ) -> Result<(), Error> {
        self.div
            .append_element::<"div">()?
            .append_text(field.name)?;
        self.div.append_child(&value.clone().node())?;
        self.fields.borrow_mut().push(Box::new(ValueFieldEditor {
            inner: value,
            field,
        }));
        Ok(())
    }
}

impl<S: 'static + Default> ValueEditor<S> for StructEditor<S> {
    fn node(self: Rc<Self>) -> Node {
        self.div.clone().into()
    }

    fn set_value(self: Rc<Self>, value: &S) {
        for field in &*self.fields.borrow() {
            field.set_value(value);
        }
    }

    fn get_value(self: Rc<Self>) -> Result<S, Error> {
        let mut result = S::default();
        for field in &*self.fields.borrow() {
            field.get_value(&mut result)?;
        }
        Ok(result)
    }
}
