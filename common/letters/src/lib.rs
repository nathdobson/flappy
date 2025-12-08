#![no_std]

use unicode_segmentation::UnicodeSegmentation;

pub static LETTERS: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ$&#0123456789:.-?!";

pub fn letters_iter() -> impl Iterator<Item = &'static str> {
    UnicodeSegmentation::graphemes(LETTERS, true)
}
