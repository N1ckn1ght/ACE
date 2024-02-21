 #![feature(test)]

extern crate test;

mod gen;
mod frame;
mod engine;

use std::io;
use crate::frame::util::visualise;
use crate::gen::{leaping::init_leaping_attacks, magic::init_magics, secondary::init_secondary_maps};
use crate::frame::board::Board;
use crate::engine::chara::Chara;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    init_secondary_maps();

    let mut board = Board::default();
    let mut chara = Chara::init(&mut board);

    println!("\n--- AKIRA HAS BEEN FULLY LOADED INTO MACHINE MEMORY ---\n");

    loop {
        println!("Processing...\n");
        break;
    }
}

fn input() -> String {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_goes_into_input_above) => {},
        Err(_no_updates_is_fine) => {},
    }
    input.trim().to_string()
}