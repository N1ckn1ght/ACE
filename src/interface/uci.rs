use std::{collections::HashSet, io, sync::mpsc::{Receiver, channel}, thread, time::Duration};
use once_cell::sync::Lazy;
use crate::{engine::{hc_eval::HCEval, search::Search}, frame::util::*};


pub fn uci_loop() -> bool {
    println!("id name {}", MYNAME);
    println!("id author {}", AUTHOR);
    
    let (tx, rx) = channel();
    let mut engine = Search::init(HCEval::init(None), &rx);
    println!("uciok");

    let mut quit = false;
    let handle = thread::spawn(move || {
        loop {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_goes_into_input_above) => {
                    let line = input.to_ascii_lowercase();
                    if line.trim() == "quit" {
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
            thread::sleep(Duration::from_micros(1));
        }
    });

    listen(&mut engine, &rx);

    let _ = handle.join();
    true
}

fn listen(engine: &mut Search, rx: &Receiver<String>) {
    loop {
        thread::sleep(Duration::from_micros(1));

        let last = rx.try_recv();
        if last.is_err() {
            continue;
        }

        let line = last.unwrap().to_ascii_lowercase();
        let cmd = line.trim().split(' ').collect::<Vec<&str>>();
        match cmd[0] {
            
            "isready" => {
                println!("readyok");
            },

            "setoption" => {
                
            },

            "ucinewgame" => {
                engine.clear();
            },

            "position" => {
                if cmd.len() < 2 {
                    engine.set_pos(None);
                    continue;
                }
                match cmd[1] {
                    "fen" => {
                        if cmd.len() < 3 {
                            engine.set_pos(None);
                            continue;
                        }
                        engine.set_pos(Some(cmd[2]));
                        if cmd.len() < 4 {
                            continue;
                        }
                        if cmd[3] != "moves" {
                            continue;
                        }
                        if cmd.len() < 5 {
                            continue;
                        }
                        parse_apply_moves(&cmd[4..], engine);
                    },
                    "startpos" => {
                        engine.set_pos(None);
                        if cmd.len() < 4 {
                            continue;
                        }
                        if cmd[2] != "moves" {
                            log(&format!("Error (unexpected argument {})", cmd[2]));
                            continue;
                        }
                        parse_apply_moves(&cmd[3..], engine);
                    },
                    "moves" => {
                        engine.set_pos(None);
                        if cmd.len() > 2 {
                            parse_apply_moves(&cmd[2..], engine);
                        }
                    },
                    _ => {
                        log(&format!("Error (unexpected argument {})", cmd[1]));
                    }
                }
            },

            "go" => {

            },
            
            "glm" => {
                let mvs = engine.get_legal_moves();
                for mv in mvs {
                    println!("{}", move_transform(mv, engine.get_turn()));
                }
            },

            "quit" => {
                return;
            }

            _ => {

            }
        }
    }
}

fn parse_apply_moves(moves: &[&str], engine: &mut Search) {
    for mov in moves {
        engine.make_move(move_transform_back(
            *mov,
            &engine.get_pseudo_legal_moves(),
            engine.get_turn()).unwrap());
    }
}

static GO_ARGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "searchmoves",
        "pomnder",
        "wtime",
        "btime",
        "winc",
        "binc",
        "movestogo",
        "depth",
        "nodes",
        "mate",
        "movetime",
        "infinite"
    ])
});