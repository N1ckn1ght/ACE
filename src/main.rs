mod gen;
mod frame;
mod engine;

use std::time::Duration;
use std::{io, thread};
use std::sync::mpsc::channel;
use crate::engine::hc_eval::HCEval;
use crate::engine::search::Search;

fn main() {
    // don't create files
    // init_magics(&mut 1773); // good random number!
    // init_leaping_attacks();
    // init_secondary_maps();

    // wait for uci declaration?
    //
    let (tx, rx) = channel();
    let mut chara = Search::init("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", rx, HCEval::init(None));

    // let handle = thread::spawn(move || {
    //     loop {
    //         let mut input = String::new();
    //         let mut quit = false;
    //         match io::stdin().read_line(&mut input) {
    //             Ok(_goes_into_input_above) => {
    //                 if input.trim() == "quit" {
    //                     quit = true;
    //                 }
    //                 let _ = tx.send(input);
    //             }
    //             Err(_no_updates_is_fine) => {
    
    //             }
    //         }
    //         if quit {
    //             break;
    //         }
    //         thread::sleep(Duration::from_millis(1));
    //     }
    // });

    // let _ = handle.join();
}