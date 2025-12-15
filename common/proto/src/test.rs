use crate::{FlappyMessage, FlappyRequest};
use core::str::FromStr;

#[cfg(feature = "serde")]
#[test]
fn test() {
    let original = FlappyMessage::Request(FlappyRequest::Run(
        heapless::String::from_str("hi").unwrap(),
    ));
    let encoded = serde_json_core::to_string::<_, 128>(&original).unwrap();
    assert_eq!(encoded, r#"{"Request":{"Run":"hi"}}"#);
    let decoded = serde_json_core::from_str::<FlappyMessage>(&encoded).unwrap().0;
    assert_eq!(original, decoded);
}
