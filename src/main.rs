mod gen;
mod frame;
mod engine;
mod interface;

use std::io::stdin;
use crate::interface::{uci::uci_loop, xboard::xboard_loop};


fn main() {
    let mut quit = false;
    loop {
        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(_goes_into_input_above) => {
                let line = input.to_lowercase();
                match line.trim() {
                    "quit" => {
                        quit = true;
                    },
                    "uci" => {
                        quit = uci_loop();
                    },
                    "xboard" => {
                        quit = xboard_loop();
                    }
                    _ => {

                    }
                };
            }
            Err(_no_updates_is_fine) => {

            }
        }
        if quit {
            break;
        }
    }
}