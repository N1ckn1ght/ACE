#![feature(test)]

extern crate test;

mod util;
mod gen;
mod board;
mod weights;
mod chara;
mod engine;

use std::io;

use chara::Chara;

use crate::board::Board;
use crate::gen::secondary::init_secondary_maps;
use crate::gen::{magic::init_magics, leaping::init_leaping_attacks};
use crate::util::*;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    init_secondary_maps();
    println!("\n--- AKIRA HAS BEEN FULLY LOADED INTO MACHINE MEMORY ---\n");

    let mut board = Board::default();
    let mut chara = Chara::init(&board, 0.08, 0.8);

    driver(&mut chara, &mut board);
}

fn driver(chara: &mut Chara, board: &mut Board) {
    let mut last_eval = Eval::new(0.0, 0, 0);
    loop {
        println!("Processing...\n");

        let legals = board.get_legal_moves();
        if legals.len() == 0 {
            break;
        }

        let ems = chara.think(board, 4, last_eval);
        for (i, em) in ems.iter().enumerate() {
            println!("{}.\t{}\tmate = {} \tscore = {}", i, move_transform(em.mov), em.eval.mate, em.eval.score);
        }
        println!();

        loop {
            let str = input();
            let mov = move_transform_back(&str.to_owned(), &legals);
            if mov.is_some() {
                chara.make_move(board, mov.unwrap());
                break;
            } else {
                println!("Move not found?");
            }
        }

        // ???
        last_eval = ems[0].eval;

        let legals = board.get_legal_moves();
        if legals.len() == 0 {
            break;
        }

        loop {
            let str = input();
            let mov = move_transform_back(&str.to_owned(), &legals);
            if mov.is_some() {
                chara.make_move(board, mov.unwrap());
                break;
            } else {
                println!("Move not found?");
            }
        }
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