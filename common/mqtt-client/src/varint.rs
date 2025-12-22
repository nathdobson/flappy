use heapless::Vec;

pub fn encode_varint(mut input: u32) -> Vec<u8, 4> {
    let mut result = Vec::<u8, 4>::new();
    loop {
        let next_data = (input & 0b111_1111) as u8;
        input = input >> 7;
        if input == 0 {
            result.push(next_data).unwrap();
            break;
        } else {
            result.push(next_data | 0b1000_0000).unwrap();
        }
    }
    result
}
