use std::{collections::HashSet, io, sync::mpsc::{Receiver, channel}, thread, time::Duration};
use once_cell::sync::Lazy;
use crate::{engine::{clock::calc_time_to_think, hc_eval::HCEval, search::Search}, frame::util::*};


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
    let mut ponder_move = "";
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
                engine.clear_history();
                engine.set_pos(None);
            },
            "position" => {
                if cmd.len() < 2 {
                    log("Error (this command requires arguments): position");
                    continue;
                }
                match cmd[1] {
                    "fen" => {
                        if cmd.len() < 3 {
                            log("Error (this argument requires parameter): fen");
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
                            log(&format!("Error (unexpected argument): {}", cmd[2]));
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
                        log(&format!("Error (unexpected argument): {}", cmd[1]));
                    }
                }
            },
            "go" => {
                let mut wtime: Option<u64> = None;
                let mut btime: Option<u64> = None;
                let mut winc: Option<u64> = None;
                let mut binc: Option<u64> = None;
                let mut movestogo: Option<u64> = None;
                let mut forcetime: u128 = 0;
                let mut node_limit: u64 = 0;
                let mut depth_limit: i16 = 0;
                let mut mate_flag = false;
                // ponder
                // searchmoves

                let mut last_arg_index = 1;
                for (i, arg) in cmd.iter().skip(2).enumerate() {
                    if GO_ARGS.contains(arg) || i + 1 == cmd.len() {
                        match cmd[last_arg_index] {
                            "searchmoves" => {
                                log("Error (not supported): searchmoves");
                            },
                            "ponder" => {
                                ponder_move = cmd[i - 1];
                            },
                            "wtime" => {
                                wtime = Some(cmd[i - 1].parse::<u64>().unwrap());
                            },
                            "btime" => {
                                btime = Some(cmd[i - 1].parse::<u64>().unwrap());
                            },
                            "winc" => {
                                winc = Some(cmd[i - 1].parse::<u64>().unwrap());
                            },
                            "binc" => {
                                binc = Some(cmd[i - 1].parse::<u64>().unwrap());
                            },
                            "movestogo" => {
                                movestogo = Some(cmd[i - 1].parse::<u64>().unwrap());
                            },
                            "depth" => {
                                depth_limit = cmd[i - 1].parse::<i16>().unwrap();
                            },
                            "nodes" => {
                                node_limit = cmd[i - 1].parse::<u64>().unwrap();
                            },
                            "mate" => {
                                depth_limit = cmd[i - 1].parse::<i16>().unwrap();
                                mate_flag = true;
                            },
                            "movetime" => {
                                forcetime = cmd[i - 1].parse::<u128>().unwrap();
                            },
                            "infinite" => {
                                forcetime = u64::MAX as u128;
                            },
                            _ => {
                                // log(&format!("Error (not supported): {}", cmd[last_arg_index]));  -- unreachable
                            }
                        };
                        last_arg_index = i;
                    }
                }

                if forcetime == 0 {
                    forcetime = calc_time_to_think(engine.get_turn(), wtime, btime, winc, binc, movestogo);
                }
                if mate_flag {
                    let result = engine.go((u64::MAX) as u128, depth_limit, 0, true);
                }
            },
            "glm" => {
                let mvs = engine.get_legal_moves();
                for mv in mvs {
                    print!("{} ", move_transform(mv, engine.get_turn()));
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
        "ponder",
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