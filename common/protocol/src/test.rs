#[cfg(feature = "serde")]
#[test]
fn test() {
    use crate::display::{DisplayMessage, DisplayRequest};
    use core::str::FromStr;
    let original = DisplayMessage::Request(DisplayRequest::Run(
        heapless::String::from_str("hi\"").unwrap(),
    ));
    let encoded = serde_json_core::to_string::<_, 128>(&original).unwrap();
    assert_eq!(encoded, r#"{"Request":{"Run":"hi\""}}"#);
    let decoded = serde_json_core::from_str_escaped::<DisplayMessage>(&encoded, &mut [0u8; 100])
        .unwrap()
        .0;
    assert_eq!(original, decoded);
}
