use crate::error::Error;
use crate::utils::{create_element, AppendChild};
use crate::value_editor::ValueEditor;
use std::marker::PhantomData;
use web_sys::{HtmlDivElement, Node};

trait FieldEditor<S: 'static> {
    fn node(&self) -> &Node;
    fn set_value(&mut self, value: &S);
    fn get_value(&self, value: &mut S) -> Result<(), Error>;
}

struct ValueFieldEditor<S, F> {
    inner: Box<dyn ValueEditor<F>>,
    field: Field<S, F>,
}

impl<S: 'static, F: 'static> FieldEditor<S> for ValueFieldEditor<S, F> {
    fn node(&self) -> &Node {
        self.inner.node()
    }

    fn set_value(&mut self, value: &S) {
        self.inner.set_value((self.field.get)(value));
    }

    fn get_value(&self, value: &mut S) -> Result<(), Error> {
        *(self.field.get_mut)(value) = self.inner.get_value()?;
        Ok(())
    }
}

pub struct StructEditor<S> {
    phantom: PhantomData<S>,
    div: HtmlDivElement,
    fields: Vec<Box<dyn FieldEditor<S>>>,
}

pub struct Field<S, F> {
    pub name: &'static str,
    pub get: Box<dyn Fn(&S) -> &F>,
    pub get_mut: Box<dyn Fn(&mut S) -> &mut F>,
}

#[macro_export]
macro_rules! field {
    ($name:ident) => {
        $crate::value_editor::struct_editor::Field {
            name: stringify!($name),
            get: Box::new(|x| &x.$name),
            get_mut: Box::new(|x| &mut x.$name),
        }
    };
}

impl<S: 'static> StructEditor<S> {
    pub fn new() -> Result<Self, Error> {
        let div = create_element::<"div">()?;
        div.set_class_name("struct-editor");
        Ok(StructEditor {
            div,
            phantom: PhantomData,
            fields: vec![],
        })
    }
    pub fn add<F: 'static>(&mut self, field: Field<S, F>, value: impl ValueEditor<F>) ->Result<(), Error>{
        let value = Box::new(value);
        self.div.append_element::<"div">()?.append_text(field.name)?;
        self.div.append_child(value.node())?;
        self.fields.push(Box::new(ValueFieldEditor {
            inner: value,
            field,
        }));
        Ok(())
    }
}

impl<S: 'static + Default> ValueEditor<S> for StructEditor<S> {
    fn node(&self) -> &Node {
        &self.div
    }

    fn set_value(&mut self, value: &S) {
        for field in &mut self.fields {
            field.set_value(value);
        }
    }

    fn get_value(&self) -> Result<S, Error> {
        let mut result = S::default();
        for field in &self.fields {
            field.get_value(&mut result)?;
        }
        Ok(result)
    }
}
