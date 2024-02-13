 #![feature(test)]

use std::io;
use crate::frame::{board::Board, util::*};
use crate::gen::{leaping::init_leaping_attacks, magic::init_magics, secondary::init_secondary_maps};
use crate::engine::chara::Chara;

extern crate test;

mod gen;
mod frame;
mod engine;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    init_secondary_maps();
    println!("\n--- AKIRA HAS BEEN FULLY LOADED INTO MACHINE MEMORY ---\n");

    let mut board = Board::default();
    let mut chara = Chara::init(&board, 0.8, 1.2, 0.0);

    driver(&mut chara, &mut board);
}

fn driver(chara: &mut Chara, board: &mut Board) {
    let mut last_eval = EvalBr::new(0.0, 0);

    loop {
        println!("Processing...\n");

        let legals = board.get_legal_moves();
        if legals.is_empty() {
            break;
        }

        let ems = chara.think(board, 0.3, 2000, last_eval);
        for (i, em) in ems.iter().enumerate() {
            println!("{}.\t{}\tscore = {}\t\tdepth = {}", i + 1, move_transform(em.mov), em.eval.score, em.eval.depth / 2);
        }
        println!();

        let mut mov;
        loop {
            let str = input();
            mov = move_transform_back(&str.to_owned(), &legals);
            if let Some(i) = mov {
                chara.make_move(board, i);
                break;
            }
            println!("Move not found?");
        }

        for em in ems.iter() {
            if em.mov == mov.unwrap() {
                last_eval = em.eval;
                break;
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