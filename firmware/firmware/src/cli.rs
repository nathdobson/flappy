use core::fmt;
use core::fmt::{Arguments, Display, Formatter};
use core::marker::PhantomData;
use core::str::Split;
use heapless::String;

#[derive(Debug)]
pub enum Adjustment {
    Add,
    Sub,
    Set,
}

#[derive(Debug)]
pub enum WifiField {
    Ssid,
    Password,
}

#[derive(Debug)]
pub enum MqttField {
    Hostname,
    Port,
    Username,
    Password,
    Topic,
}

#[derive(Debug)]
pub enum TestType {
    Read,
    Enable,
    Spin,
}

#[derive(Debug)]
pub enum Command<'a> {
    Phantom(!, PhantomData<&'a ()>),
    // WifiRead,
    // WifiWrite(WifiField, &'a str),
    // MqttRead,
    // MqttWrite(MqttField, &'a str),
    // CalibrateRead,
    // CalibrateReadOne(usize),
    // CalibrateWriteOne(usize, Adjustment, usize),
    Help,
    Test(TestType),
}

pub struct CommandError(String<32>);

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> From<fmt::Arguments<'a>> for CommandError {
    fn from(value: Arguments<'a>) -> Self {
        use core::fmt::Write;
        let mut result = String::new();
        write!(&mut result, "{}", value).ok();
        CommandError(result)
    }
}

impl<'a> From<&'a str> for CommandError {
    fn from(value: &'a str) -> Self {
        Self::from(format_args!("{}", value))
    }
}

impl<'a> Command<'a> {
    pub fn parse(input: &'a str) -> Result<Command<'a>, CommandError> {
        let mut input = input.split(" ");
        let kind = input
            .next()
            .ok_or::<CommandError>("no command given".into())?;
        match kind {
            "help" => Self::parse_help(input),
            // "calibrate" => Self::parse_calibrate(input),
            // "display" => Self::parse_display(input),
            // "wifi" => Self::parse_wifi(input),
            // "mqtt" => Self::parse_mqtt(input),
            "test" => Self::parse_test(input),
            _ => Err(format_args!("unknown command '{}'", kind).into()),
        }
    }

    fn parse_help(mut input: Split<'a, &'a str>) -> Result<Command<'a>, CommandError> {
        if input.next().is_some() {
            return Err("unexpected arguments".into());
        }
        Ok(Command::Help)
    }

    // fn parse_calibrate(mut input: Split<'a, &'a str>) -> Result<Command<'a>, CommandError> {
    //     let index = match input.next() {
    //         None => return Ok(Command::CalibrateRead),
    //         Some(index) => index,
    //     };
    //     let index = match index.parse::<usize>() {
    //         Ok(index) => index,
    //         Err(e) => return Err(format_args!("Cannot parse index '{}': {}", index, e).into()),
    //     };
    //     let value = match input.next() {
    //         None => return Ok(Command::CalibrateReadOne(index)),
    //         Some(value) => value,
    //     };
    //     let (adj, value) = if let Some(add) = value.strip_prefix("+") {
    //         (Adjustment::Add, add)
    //     } else if let Some(sub) = value.strip_prefix("-") {
    //         (Adjustment::Sub, sub)
    //     } else {
    //         (Adjustment::Set, value)
    //     };
    //     let value = match value.parse::<usize>() {
    //         Ok(index) => index,
    //         Err(e) => return Err(format_args!("Cannot parse value '{}': {}", value, e).into()),
    //     };
    //     Ok(Command::CalibrateWriteOne(index, adj, value))
    // }

    // fn parse_display(mut input: Split<'a, &'a str>) -> Result<Command<'a>, CommandError> {
    //     Ok(Command::Display(input.remainder().unwrap_or("")))
    // }

    // fn parse_wifi(mut input: Split<'a, &'a str>) -> Result<Command<'a>, CommandError> {
    //     let field = match input.next() {
    //         None => return Ok(Command::WifiRead),
    //         Some(field) => field,
    //     };
    //     let field = match field {
    //         "ssid" => WifiField::Ssid,
    //         "password" => WifiField::Password,
    //         _ => return Err(format_args!("Unrecognized field {}", field).into()),
    //     };
    //     let value = match input.next() {
    //         None => return Err(format_args!("Missing value").into()),
    //         Some(value) => value,
    //     };
    //     Ok(Command::WifiWrite(field, value))
    // }

    // fn parse_mqtt(mut input: Split<'a, &'a str>) -> Result<Command<'a>, CommandError> {
    //     let field = match input.next() {
    //         None => return Ok(Command::MqttRead),
    //         Some(field) => field,
    //     };
    //     let field = match field {
    //         "hostname" => MqttField::Hostname,
    //         "port" => MqttField::Port,
    //         "username" => MqttField::Username,
    //         "password" => MqttField::Password,
    //         "topic" => MqttField::Topic,
    //         _ => return Err(format_args!("Unrecognized field {}", field).into()),
    //     };
    //     let value = match input.next() {
    //         None => return Err(format_args!("Missing value").into()),
    //         Some(value) => value,
    //     };
    //     Ok(Command::MqttWrite(field, value))
    // }
    fn parse_test(mut input: Split<'a, &'a str>) -> Result<Command<'a>, CommandError> {
        let typ = input
            .next()
            .ok_or(CommandError::from("missing test type"))?;
        let typ = match typ {
            "spin" => TestType::Spin,
            "read" => TestType::Read,
            "enable" => TestType::Enable,
            _ => return Err(CommandError::from("unknown test type")),
        };
        Ok(Command::Test(typ))
    }
}
