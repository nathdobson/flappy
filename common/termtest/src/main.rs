#![deny(unused_must_use)]

use std::io;
use std::io::Write;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

const SAVE: &'static str = "\x1B[s";
const RESTORE: &'static str = "\x1B[u";
const GOTO0: &'static str = "\x1B[1;1H";
const GOTO1: &'static str = "\x1B[10;1H";
const GOTO2: &'static str = "\x1B[9999;1H";
const REGION1: &'static str = "\x1B[2;10r";
const REGION2: &'static str = "\x1B[11;r";
const ERASE: &'static str = "\x1B[2J";
const UP_ONE:&'static str= "\x1B[A";
const INSERT_LINE:&'static str= "\x1B[L";
const DELETE_LINE:&'static str= "\x1B[10M";
const SCROLL:&'static str= "\x1B[1S";
fn main() {
    print!("{ERASE}{GOTO2}");
    io::stdout().flush().unwrap();
    loop {
        print!("{SAVE}{GOTO2}aaa\nbbb\nccc{RESTORE}{SCROLL}");
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(1000));
        // print!("{SAVE}{REGION1}{GOTO1}{:?}\n{RESTORE}", SystemTime::now());
        // io::stdout().flush().unwrap();
        // sleep(Duration::from_millis(1000));
        // print!("{SAVE}{REGION2}{GOTO2}{:?}\n{RESTORE}", SystemTime::now());
        // io::stdout().flush().unwrap();
        // sleep(Duration::from_millis(1000));
    }
}

#[test]
fn test() {}
