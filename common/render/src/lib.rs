#![no_std]
#![feature(iter_order_by)]
#![deny(unused_must_use)]
#![allow(unused_imports)]

use core::iter;
use heapless::{CapacityError, Vec};
#[cfg(feature = "unicode")]
use unicode_normalization::{UnicodeNormalization, try_iter_eq, try_iter_eq_by};
#[cfg(feature = "unicode")]
use unicode_segmentation::UnicodeSegmentation;
#[cfg(feature = "unicode")]
use unidecode::unidecode_char;

pub struct Renderer<'a, const CAP: usize> {
    ideals: &'a [&'a str],
    glyphs: Vec<usize, CAP>,
}

impl<'a, const CAP: usize> Renderer<'a, CAP> {
    pub fn new(ideals: &'a [&'a str]) -> Self {
        Renderer {
            ideals,
            glyphs: Vec::new(),
        }
    }
    pub fn append(&mut self, text: &str) -> Result<(), CapacityError> {
        #[cfg(not(feature = "unicode"))]
        for c in text.chars() {
            self.append_match(|s| {
                Ok(s.len() == 1 && s.chars().next().unwrap().eq_ignore_ascii_case(&c))
            })?;
        }
        #[cfg(feature = "unicode")]
        for grapheme in UnicodeSegmentation::graphemes(text, true) {
            self.append_grapheme(grapheme)?;
        }
        Ok(())
    }
    fn append_match(
        &mut self,
        p: impl Fn(&str) -> Result<bool, CapacityError>,
    ) -> Result<bool, CapacityError> {
        for (index, ideal) in self.ideals.iter().enumerate() {
            if p(*ideal)? {
                self.glyphs
                    .push(index)
                    .map_err(|_| CapacityError::default())?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    #[cfg(feature = "unicode")]
    fn eq_ignore_case(c1: char, c2: char) -> Result<bool, CapacityError> {
        Ok(c1.to_lowercase().eq(c2.to_lowercase()))
    }
    #[cfg(feature = "unicode")]
    fn append_grapheme(&mut self, grapheme: &str) -> Result<(), CapacityError> {
        if self.append_match(|g| Ok(g == grapheme))? {
            return Ok(());
        }
        // Check for a matching grapheme with the same Unicode canonical normalization. This ensures
        // graphemes with different code point sequences that should render identically are
        // matched. For example, "\u00F1" (LATIN SMALL LETTER N WITH TILDE) and "\u006E\u0303"
        // (LATIN SMALL LETTER N, COMBINING TILDE) should both use the same flap.
        if self.append_match(|g| try_iter_eq(g.nfd(), grapheme.nfd()))? {
            return Ok(());
        }
        // If we failed to find a canonical match, look for a compatible match. This will handle
        // imperfect matches like "\u0190" (LATIN CAPITAL LETTER OPEN E ) for "\u2107" (EULER CONSTANT).
        if self.append_match(|g| try_iter_eq(g.nfkd(), grapheme.nfkd()))? {
            return Ok(());
        }
        // Repeat ignoring case
        if self.append_match(|g| {
            try_iter_eq_by(
                g.chars().map(Ok),
                grapheme.chars().map(Ok),
                Self::eq_ignore_case,
            )
        })? {
            return Ok(());
        }
        if self.append_match(|g| try_iter_eq_by(g.nfd(), grapheme.nfd(), Self::eq_ignore_case))? {
            return Ok(());
        }
        if self.append_match(|g| try_iter_eq_by(g.nfkd(), grapheme.nfkd(), Self::eq_ignore_case))? {
            return Ok(());
        }
        let mut success = false;
        // We failed to find a match for the entire grapheme, but individual codepoints might match.
        for c in grapheme.nfkd() {
            let c = c?;
            if self.append_match(|g| Ok(g.chars().eq(iter::once(c))))? {
                success = true;
                continue;
            }
            if self.append_match(|g| {
                try_iter_eq_by(g.chars().map(Ok), iter::once(Ok(c)), Self::eq_ignore_case)
            })? {
                success = true;
                continue;
            }
            // If we can't find a match in Unicode, it might exist in ASCII.
            for c in unidecode_char(c).chars() {
                if self.append_match(|g| Ok(g.chars().eq(iter::once(c))))? {
                    success = true;
                    continue;
                }
                if self.append_match(|g| {
                    try_iter_eq_by(g.chars().map(Ok), iter::once(Ok(c)), Self::eq_ignore_case)
                })? {
                    success = true;
                    continue;
                }
            }
        }
        if !success {
            // If all else fails, generate a placeholder.
            self.glyphs.push(0).ok();
        }
        Ok(())
    }
    pub fn finish(self) -> Vec<usize, CAP> {
        self.glyphs
    }
}

#[test]
fn test_exact() {
    let mut renderer = Renderer::<10>::new(&[
        " ",
        "n",
        "\u{00F1}",
        "n\u{0303}",
        "N",
        "\u{00D1}",
        "N\u{0303}",
    ]);
    renderer
        .append("n\u{00F1}n\u{0303}N\u{00D1}N\u{0303}")
        .unwrap();
    assert_eq!(renderer.finish(), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_equivalent() {
    let mut renderer = Renderer::<10>::new(&[" ", "n", "n\u{0303}", "N", "N\u{0303}"]);
    renderer
        .append("n\u{00F1}n\u{0303}N\u{00D1}N\u{0303}")
        .unwrap();
    assert_eq!(renderer.finish(), [1, 2, 2, 3, 4, 4]);
}

#[test]
fn test_equivalent_case_insensitive() {
    let mut renderer = Renderer::<10>::new(&[" ", "n\u{0303}", "N", "N\u{0303}"]);
    renderer
        .append("n\u{00F1}n\u{0303}N\u{00D1}N\u{0303}")
        .unwrap();
    assert_eq!(renderer.finish(), [2, 1, 1, 2, 3, 3]);
}

#[test]
fn test_compatible() {
    let mut renderer = Renderer::<10>::new(&[" ", "\u{0190}"]);
    renderer.append("\u{0190}\u{2107}").unwrap();
    assert_eq!(renderer.finish(), [1, 1]);
    let mut renderer = Renderer::<10>::new(&[" ", "\u{2107}"]);
    renderer.append("\u{0190}\u{2107}").unwrap();
    assert_eq!(renderer.finish(), [1, 1]);
}

#[test]
fn test_partial_match() {
    let mut renderer = Renderer::<10>::new(&[" ", "n", "N"]);
    renderer
        .append("n\u{00F1}n\u{0303}N\u{00D1}N\u{0303}")
        .unwrap();
    assert_eq!(renderer.finish(), [1, 1, 1, 2, 2, 2]);
}

#[test]
fn test_partial_match_case_insensitive() {
    let mut renderer = Renderer::<10>::new(&[" ", "n"]);
    renderer
        .append("n\u{00F1}n\u{0303}N\u{00D1}N\u{0303}")
        .unwrap();
    assert_eq!(renderer.finish(), [1, 1, 1, 1, 1, 1]);
}

#[test]
fn test_unidecode() {
    let mut renderer = Renderer::<10>::new(&[" ", "b", "e", "i", "j", "n", "g"]);
    renderer.append("北亰").unwrap();
    assert_eq!(renderer.finish(), [1, 2, 3, 0, 4, 3, 5, 6, 0]);
}

#[test]
fn test_zalgo() {
    let mut renderer = Renderer::<100>::new(&[" "]);
    renderer.append("Í̴͚̗̤̘̠̘̥̗̣̊́̀͋̐͂͌̂̈́̄̊̿͘̕͝͝ ̵̢̬̞̥͙̦̠̘̠̘̩͊̉̅̆̈́̈̏͗̋͊̑͜͠h̴̡̭̖̍̃͑̐͒́̾̔̉̂̀̑̋͘͠ą̸̰͉̣͎̼̦͕̗̲̝͚̭̠͖̄̈̓͒͊̓͒̈̕͠v̶̨̛͚̥̻̼̘͆̇̇̿̎̊͝ͅe̶̺̋̈́̑̆͆͋̽̂͐͌̾̕͘̚͠ ̸̨̦͚̰̥̟̟̼̲̩̣͚͚͕̑̎̾̆̌͘͜͝b̸̙̭̞͉̠̹͓̺̪̒̈́͛̈́͋̓͝͝ę̶͔͕͔͍͇̥̠̙̙̭̻̺̟̬̓̈́̅͌c̷̛͇̻̺̘̜͙̥̫͌̌̓͑̓͑͝ơ̸̺̇̓͂͐̆̒̿̑͌̚̚͜m̵̳͉͚͋͑̀̀͜ȩ̸̨̨̛͕̙̟̠̪̲̪̯̄͋̉͆̾͂̊̚͝ ̵̢̨̢̛̪̹̩̜̲̤̞̩͈͐͂͗̾̎̒̄̿o̴̡̬͍̪͈̙̿̓v̴̛̤͇̱̼̯̱̱̪̞͔̦͐̾̂͊̓̔͐̊̓͒̋͌͘͝ȩ̸̨̛͔͙̥̺̟̋̒̒͛̂̄͂̂̒͌̈̓̆̓̚̚r̶̛̖̯͖̠̦͋͆̄̉f̴̰͕͙̯͖̪̩͙̭̙̗͇͕͂̀́͒̑̐̈́̏͊̽l̶̪͙͖̦̏̈́̔̑̐͐͆̚o̶̦̝̅̃͌͑̑̀̿̐̅̾̓͒́̊͗̚ẅ̸̢̢̳̘̮͇̙̣͈͍̮͓̫̘́̂̈́̈͘͘").unwrap_err();
    assert_eq!(renderer.finish(), []);
}
