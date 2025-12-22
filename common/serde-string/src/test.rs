use crate::{from_str, to_string};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[test]
fn test() {
    const LITERAL: &str = "ABC";
    fn test<'de, T: Serialize + Deserialize<'de> + Eq + Debug>(x: T) {
        assert_eq!(to_string(&x).unwrap(), LITERAL);
        assert_eq!(&from_str::<T>(&LITERAL).unwrap(), &x);
    }
    test(LITERAL);
    test(LITERAL.to_owned());
    test((LITERAL,));
    test([LITERAL]);
    test(vec![LITERAL]);
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Newtype(String);
    test(Newtype(LITERAL.to_owned()));
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Struct1 {
        x: String,
    }
    test(Struct1 {
        x: LITERAL.to_owned(),
    });
}
