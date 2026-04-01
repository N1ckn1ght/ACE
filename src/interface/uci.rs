use std::{collections::HashSet, io::stdin, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{Sender, channel}}, thread, time::Duration};
use once_cell::sync::Lazy;
use crate::{engine::{clock::calc_time_to_think, hc_eval::HCEval, search::Search}, frame::util::*};


pub fn uci_loop() -> bool {
    println!("id name {}", MYNAME);
    println!("id author {}", AUTHOR);

    let (tx, rx) = channel::<String>();  // this is not optimal, this is bad
    let abort = Arc::new(AtomicBool::new(true));
    let abort_listener_clone = Arc::clone(&abort);

    let handle = thread::spawn(move || {
        let mut engine = Search::init();
        engine.abort = Arc::clone(&abort);
        let eval = HCEval::init(None);
        println!("option name Hash type spin default 384 min 1 max 24576");
        println!("uciok");

        for input in rx {
            let cmd = input.trim().split_whitespace().collect::<Vec<&str>>();
            match cmd[0] {
                "setoption" => {
                    // TODO
                },
                "ucinewgame" => {
                    engine.clear_cache();
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
                            parse_apply_moves(&cmd[4..], &mut engine);
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
                            parse_apply_moves(&cmd[3..], &mut engine);
                        },
                        "moves" => {
                            engine.set_pos(None);
                            if cmd.len() > 2 {
                                parse_apply_moves(&cmd[2..], &mut engine);
                            }
                        },
                        _ => {
                            log(&format!("Error (unexpected argument): {}", cmd[1]));
                        }
                    }
                },
                "go" => {
                    let mut movetime: Option<u64> = None;
                    let mut wtime: Option<u64> = None;
                    let mut btime: Option<u64> = None;
                    let mut winc: Option<u64> = None;
                    let mut binc: Option<u64> = None;
                    let mut movestogo: Option<u64> = None;
                    let mut node_limit: u64 = 0;
                    let mut depth_limit: u8 = 127;
                    let mut mate_flag = false;
                    let mut searchmoves: Option<&[&str]> = None;
                    // ponder

                    let mut last_arg_index = cmd.len();
                    for (i, arg) in cmd.iter().enumerate().skip(1).rev() {
                        if GO_ARGS.contains(arg) {
                            match cmd[i] {
                                "searchmoves" => {
                                    searchmoves = Some(&cmd[i+1..last_arg_index]);
                                },
                                "ponder" => {
                                    // todo
                                },
                                "wtime" => {
                                    wtime = Some(cmd[i + 1].parse::<u64>().unwrap());
                                },
                                "btime" => {
                                    btime = Some(cmd[i + 1].parse::<u64>().unwrap());
                                },
                                "winc" => {
                                    winc = Some(cmd[i + 1].parse::<u64>().unwrap());
                                },
                                "binc" => {
                                    binc = Some(cmd[i + 1].parse::<u64>().unwrap());
                                },
                                "movestogo" => {
                                    movestogo = Some(cmd[i + 1].parse::<u64>().unwrap());
                                },
                                "depth" => {
                                    depth_limit = cmd[i + 1].parse::<i32>().unwrap().clamp(1, 254) as u8;
                                },
                                "nodes" => {
                                    node_limit = cmd[i + 1].parse::<u64>().unwrap().max(1000);
                                },
                                "mate" => {
                                    depth_limit = cmd[i + 1].parse::<i32>().unwrap().clamp(1, 254) as u8;
                                    mate_flag = true;
                                    movetime = Some(INFINITE_TIME);
                                },
                                "movetime" => {
                                    movetime = Some(cmd[i + 1].parse::<u64>().unwrap());
                                },
                                "infinite" => {
                                    movetime = Some(INFINITE_TIME);
                                },
                                _ => {
                                    // -- unreachable in this implementation
                                }
                            };
                            last_arg_index = i;
                        }
                    }
                    let time = calc_time_to_think(engine.get_turn(), movetime, wtime, btime, winc, binc, movestogo);
                    log(&format!("Launching search w/ options: forcetime {} depth_limit {} node_limit {} mate_flag {} do_searchmoves {}", time, depth_limit, node_limit, mate_flag, searchmoves.is_some()));
                    let (bestmove, ponder) = engine.go(&eval, time, depth_limit, node_limit, mate_flag, searchmoves);
                    if ponder.is_some() {
                        println!("bestmove {} ponder {}", bestmove, ponder.unwrap());
                    } else {
                        println!("bestmove {}", bestmove);
                    }
                },
                "glm" => {
                    let mvs = engine.get_legal_moves();
                    for mv in mvs {
                        print!("{} ", move_transform(mv, engine.get_turn()));
                    }
                },
                "quit" => {
                    break;
                }
                _ => {

                }
            }
        }
    });

    listen(&tx, abort_listener_clone);

    let _ = handle.join();
    true
}

/// Read loop
fn listen(tx: &Sender<String>, abort: Arc<AtomicBool>) {
    loop {
        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(_goes_into_input_above) => {

            },
            Err(_no_updates_is_fine) => {
                continue;
            }
        }
        input = input.trim().to_lowercase();

        let first = input.split_whitespace().next().unwrap_or("");
        match first {
            "isready" => {
                println!("readyok");
            },
            "setoption" | "ucinewgame" | "position" | "go" => {
                abort.store(true, Ordering::Relaxed);
                let _ = tx.send(input).unwrap();
            },
            "stop" => {
                abort.store(true, Ordering::Relaxed);
            },
            "quit" => {
                abort.store(true, Ordering::Relaxed);
                let _ = tx.send(input).unwrap();
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
            engine.get_turn()).unwrap()
        );
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