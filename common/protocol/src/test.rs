#[cfg(feature = "serde")]
#[test]
fn test() {
    use crate::display::DisplayRequest;
    use core::str::FromStr;
    let original = DisplayRequest::Run(heapless::String::from_str("hi\"").unwrap());
    let encoded = serde_json_core::to_string::<_, 128>(&original).unwrap();
    assert_eq!(encoded, r#"{"Run":"hi\""}"#);
    let decoded = serde_json_core::from_str_escaped::<DisplayRequest>(&encoded, &mut [0u8; 100])
        .unwrap()
        .0;
    assert_eq!(original, decoded);
}
