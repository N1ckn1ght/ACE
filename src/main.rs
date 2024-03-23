#![feature(test)]

extern crate test;

mod gen;
mod frame;
mod engine;

use std::io;
use crate::frame::util::*;
use crate::gen::{leaping::init_leaping_attacks, magic::init_magics, secondary::init_secondary_maps};
use crate::engine::chara::Chara;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    init_secondary_maps();
    
    let mut chara = Chara::default();

    chara.w.rand = 0;

    println!("\n--- AKIRA HAS BEEN FULLY LOADED INTO MACHINE MEMORY ---\n");

    let mut hmc = 0;
    let scan = [true, true];
    let ab = [300, INF];
    // soft limit lol, may overflow by about ~50ms?
    let time = 2950;
    let dl = 50;
    let mut abi = 0;

    loop {
        let legals = chara.board.get_legal_moves();
        if legals.is_empty() {
            break;
        }

        if scan[hmc & 1] {
            println!("Processing...\n");
            let best_move = chara.think(ab[abi], time, dl);
            abi = 0;
            println!();
            println!("Best move: {} ({})", move_transform(best_move.mov, chara.board.turn), score_transform(best_move.score, chara.board.turn));
            println!();
        }

        let mut mov;
        loop {
            println!();
            let str = input();

            if str == "b" {
                chara.revert_move();
                hmc -= 2;
                break;
            } 
            if str == "r" {
                hmc -= 1;
                break;
            } 
            if str == "hr" {
                hmc -= 1;
                abi = 1;
                break;
            } 
            if str == "ex" {
                println!("{}", chara.board.export());
                continue;
            }

            mov = move_transform_back(&str.to_owned(), &legals, chara.board.turn);
            if let Some(i) = mov {
                chara.make_move(i);
                break;
            }
            println!("Move not found?");
        }

        hmc += 1;
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