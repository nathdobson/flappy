use crate::LogVec;
use embedded_io::Write;

type MyLogVec = LogVec<String, 4, 4>;

fn get_state(log_vec: &MyLogVec) -> Vec<(usize, String, Vec<u8>)> {
    log_vec
        .into_iter()
        .map(|(i, v, b)| (i, v.clone(), b.into_iter().cloned().collect::<Vec<_>>()))
        .collect::<Vec<_>>()
}

#[test]
fn test() {
    let mut log_vec = MyLogVec::new();
    let mut builder = log_vec.push_back();
    builder.write_all(b"aa").unwrap();
    builder.build("x".to_string());
    let mut builder = log_vec.push_back();
    builder.write_all(b"bb").unwrap();
    builder.build("y".to_string());
    assert_eq!(
        vec![
            (0, "x".to_string(), b"aa".to_vec()),
            (1, "y".to_string(), b"bb".to_vec())
        ],
        get_state(&log_vec)
    );
    let mut builder = log_vec.push_back();
    builder.write_all(b"cc").unwrap();
    builder.build("z".to_string());
    assert_eq!(
        vec![
            (1, "y".to_string(), b"bb".to_vec()),
            (2, "z".to_string(), b"cc".to_vec())
        ],
        get_state(&log_vec)
    );
}
