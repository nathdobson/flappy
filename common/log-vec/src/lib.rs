#![cfg_attr(not(test), no_std)]
#![feature(impl_trait_in_assoc_type)]
#![feature(debug_closure_helpers)]
extern crate alloc;

mod error;
mod ring_buffer;
#[cfg(test)]
mod ring_buffer_test;
mod slice_pair;
#[cfg(test)]
mod log_vec_test;

pub use crate::error::CapacityError;
use crate::ring_buffer::{RingBuffer, RingBufferView};
use core::ops::Range;
use embedded_io::{BufRead, ErrorType, Write};
use heapless::Deque;
use heapless::deque::DequeView;
pub use slice_pair::SlicePair;

pub struct LogEntry<P> {
    bytes: Range<usize>,
    value: P,
}

pub struct LogVec<T, const P: usize, const B: usize> {
    dropped_bytes: usize,
    dropped_packets: usize,
    bytes: RingBuffer<B>,
    packets: Deque<LogEntry<T>, P>,
}

pub struct LogEntryBuilder<'a, T> {
    dropped_bytes: &'a mut usize,
    dropped_packets: &'a mut usize,
    bytes: &'a mut RingBufferView,
    packets: &'a mut DequeView<LogEntry<T>>,
    start: usize,
    built: bool,
    len: usize,
}

impl<T, const P: usize, const B: usize> LogVec<T, P, B> {
    pub fn new() -> Self {
        LogVec {
            dropped_bytes: 0,
            dropped_packets: 0,
            bytes: RingBuffer::new(),
            packets: Deque::new(),
        }
    }
    pub fn push_back(&mut self) -> LogEntryBuilder<'_, T> {
        let start = self.dropped_bytes + self.bytes.len();
        LogEntryBuilder {
            dropped_bytes: &mut self.dropped_bytes,
            dropped_packets: &mut self.dropped_packets,
            bytes: &mut self.bytes,
            packets: &mut self.packets,
            start,
            built: false,
            len: 0,
        }
    }
    pub fn packet_range(&self) -> Range<usize> {
        self.dropped_packets..self.dropped_packets + self.packets.len()
    }
    pub fn packet(&self, index: usize) -> Option<(&'_ T, SlicePair<'_, u8>)> {
        let index = index.checked_sub(self.dropped_packets)?;
        let packet = self.packets.get(index)?;
        let bytes = self
            .bytes
            .as_slice_pair()
            .subslice(
                packet.bytes.start - self.dropped_bytes..packet.bytes.end - self.dropped_bytes,
            )
            .unwrap();
        Some((&packet.value, bytes))
    }
}

impl<'a, T, const P: usize, const B: usize> IntoIterator for &'a LogVec<T, P, B> {
    type Item = (usize, &'a T, SlicePair<'a, u8>);
    type IntoIter = impl 'a + Iterator<Item = (usize, &'a T, SlicePair<'a, u8>)>;
    fn into_iter(self) -> Self::IntoIter {
        self.packet_range().map(|i| {
            let (value, bytes) = self.packet(i).unwrap();
            (i, value, bytes)
        })
    }
}

impl<'a, T> LogEntryBuilder<'a, T> {
    fn drop_packet(&mut self) -> bool {
        let Some(next) = self.packets.pop_front() else {
            return false;
        };
        self.bytes.consume(next.bytes.len());
        *self.dropped_bytes += next.bytes.len();
        *self.dropped_packets += 1;
        true
    }
}

impl<'a, T> LogEntryBuilder<'a, T> {
    pub fn build(mut self, value: T) {
        self.built = true;
        if self.packets.is_full() {
            if !self.drop_packet() {
                return;
            }
        }
        self.packets
            .push_back(LogEntry {
                bytes: self.start..self.start + self.len,
                value,
            })
            .ok()
            .unwrap();
    }
}

impl<'a, T> Drop for LogEntryBuilder<'a, T> {
    fn drop(&mut self) {
        if !self.built {
            self.bytes.consume_back(self.len);
        }
    }
}

impl<'a, T> ErrorType for LogEntryBuilder<'a, T> {
    type Error = CapacityError;
}

impl<'a, T> Write for LogEntryBuilder<'a, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        loop {
            match self.bytes.write(buf) {
                Ok(l) => {
                    self.len += l;
                    return Ok(l);
                }
                Err(CapacityError) => {
                    if self.drop_packet() {
                        continue;
                    } else {
                        return Err(CapacityError);
                    }
                }
            }
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
