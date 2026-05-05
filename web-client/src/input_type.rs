pub enum InputType {
    Text,
    Number,
    Color,
}

impl InputType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Number => "number",
            InputType::Color => "color",
        }
    }
}
