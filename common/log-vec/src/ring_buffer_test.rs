use crate::CapacityError;
use crate::ring_buffer::RingBuffer;
use alloc::vec::Vec;
use embedded_io::{Read, Write};
use heapless::Deque;
use rand::{RngExt, SeedableRng};
use rand_xorshift::XorShiftRng;

#[test]
fn test_ring_buffer() {
    const CAP: usize = 10;
    for seed in 0..1000 {
        println!("Seed is {}", seed);
        let mut rng = XorShiftRng::seed_from_u64(seed);
        let mut heapless = Deque::<u8, CAP>::new();
        let mut ring_buffer = RingBuffer::<CAP>::new();
        for _ in 0..1000 {
            if rng.random_bool(0.5) {
                let len = rng.random_range(1..10);
                let data = (&mut rng).random_iter().take(len).collect::<Vec<u8>>();
                match ring_buffer.write(&data) {
                    Ok(len) => {
                        heapless.extend(&data[..len]);
                    }
                    Err(CapacityError) => {
                        assert_eq!(heapless.len(), CAP, "{:?}", ring_buffer);
                    }
                }
            } else {
                let len = rng.random_range(1..10);
                let mut data1 = vec![0; len];
                let read_len = ring_buffer.read(&mut data1).unwrap();
                if heapless.is_empty() {
                    assert_eq!(read_len, 0);
                } else {
                    assert_ne!(read_len, 0);
                }
                let mut data2 = vec![];
                for _ in 0..read_len {
                    data2.push(heapless.pop_front().unwrap());
                }
                assert_eq!(data1[..read_len], data2);
            }
        }
        println!("{:?} {:?}", heapless, ring_buffer);
    }
}
