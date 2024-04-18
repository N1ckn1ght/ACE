#![feature(test)]

extern crate test;

mod gen;
mod frame;
mod engine;

use std::time::Duration;
use std::{io, thread};
use std::sync::mpsc::channel;
use crate::gen::{leaping::init_leaping_attacks, magic::init_magics, secondary::init_secondary_maps};
use crate::engine::chara::Chara;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    init_secondary_maps();
    
    let (tx, rx) = channel();
    let mut chara = Chara::init("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", rx);

    let handle = thread::spawn(move || {
        loop {
            let mut input = String::new();
            let mut quit = false;
            match io::stdin().read_line(&mut input) {
                Ok(_goes_into_input_above) => {
                    if input.trim() == "quit" {
                        quit = true;
                    }
                    let _ = tx.send(input);
                }
                Err(_no_updates_is_fine) => {
    
                }
            }
            if quit {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
    });

    println!("#DEBUG\tAkira is online.\n");
    chara.listen();

    println!("#DEBUG\tShutting down!\n");
    let _ = handle.join();
}