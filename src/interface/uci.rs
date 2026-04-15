use std::{collections::HashSet, io::stdin, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{Sender, channel}}, thread};
use once_cell::sync::Lazy;
use super::*;
use crate::{engine::{clock::calc_time_to_think, hc_eval::HCEval, search::Search}, frame::util::*};


pub fn uci_loop() -> bool {
    print_greeting();
    let (tx, rx) = channel::<String>();  // not sure
    let abort = Arc::new(AtomicBool::new(true));
    let ponder = Arc::new(AtomicBool::new(false));

    let abort_listener_clone = Arc::clone(&abort);
    let ponder_listener_clone = Arc::clone(&ponder);

    let handle = thread::spawn(move || {
        let mut engine = Search::init();
        engine.abort = Arc::clone(&abort);
        engine.ponder = Arc::clone(&ponder);
        let eval = HCEval::init(None);

        for input in rx {
            let cmd = input.split_whitespace().collect::<Vec<&str>>();
            match cmd[0].to_lowercase().as_str() {
                "setoption" => {
                    if cmd.len() < 5 {
                        println!("Error (usage: setoption name OPTION value VALUE)");
                        continue;
                    }
                    match cmd[2].to_lowercase().as_str() {
                        "hash" => {
                            let mb = cmd[4].parse::<u32>().unwrap().clamp(1, 24576);
                            engine.set_cache_size(mb);
                        },
                        _ => {
                            println!("Error (no such option): {}", cmd[2]);
                        }
                    }
                },
                "ucinewgame" => {
                    engine.clear_cache();
                    engine.set_pos(None);
                },
                "position" => {
                    if cmd.len() < 2 {
                        println!("Error (this command requires arguments): position");
                        continue;
                    }
                    match cmd[1].to_lowercase().as_str() {
                        "fen" => {
                            if cmd.len() < 3 {
                                println!("Error (this argument requires parameter): fen");
                                continue;
                            }
                            // I'm gonna assume and haters gonna hate for inconsistency
                            // expected input like
                            // rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
                            // if it has no "- 0 1" part, you'll see a crash
                            let mut last_index = 3;
                            for arg in cmd.iter().skip(3) {
                                if arg == &"moves" {
                                    break;
                                }
                                last_index += 1;
                            }
                            let fen = cmd[2..last_index].join(" ");
                            // log(&format!("Setting up this fen: <{}>", fen));
                            engine.set_pos(Some(&fen));
                            if cmd.len() < last_index + 2 {
                                continue;
                            }
                            parse_apply_moves(&cmd[last_index + 1..], &mut engine);
                        },
                        "startpos" => {
                            engine.set_pos(None);
                            if cmd.len() < 4 {
                                continue;
                            }
                            if cmd[2].to_lowercase().as_str() != "moves" {
                                println!("Error (unexpected argument): {}", cmd[2]);
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
                            println!("Error (unexpected argument): {}", cmd[1]);
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
                    let mut depth_limit: u8 = HARD_DEPTH_LIMIT as u8;
                    let mut searchmoves: Option<&[&str]> = None;
                    let mut ponder = false;
                    let mut show_eval = false;

                    let mut last_arg_index = cmd.len();
                    for (i, arg) in cmd.iter().enumerate().skip(1).rev() {
                        if GO_ARGS.contains(arg.to_lowercase().as_str()) {
                            match cmd[i].to_lowercase().as_str() {
                                "searchmoves" => {
                                    searchmoves = Some(&cmd[i+1..last_arg_index]);
                                },
                                "ponder" => {
                                    ponder = true;
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
                                    depth_limit = cmd[i + 1].parse::<usize>().unwrap().clamp(1, HARD_DEPTH_LIMIT) as u8;
                                },
                                "nodes" => {
                                    node_limit = cmd[i + 1].parse::<u64>().unwrap().max(1000);
                                },
                                "movetime" => {
                                    movetime = Some(cmd[i + 1].parse::<u64>().unwrap());
                                },
                                "infinite" => {
                                    movetime = Some(INFINITE_TIME);
                                },
                                "eval" => {
                                    // custom non-uci command
                                    show_eval = true;
                                },
                                _ => {
                                    // -- unreachable in this implementation
                                }
                            };
                            last_arg_index = i;
                        }
                    }

                    engine.do_post = true;
                    engine.ponder.store(ponder, Ordering::Relaxed);
                    let time = calc_time_to_think(engine.get_turn(), movetime, wtime, btime, winc, binc, movestogo);

                    log(&format!("Launching search w/ options: forcetime {} depth_limit {} node_limit {} ponder {} do_searchmoves {}", time, depth_limit, node_limit, ponder, searchmoves.is_some()));

                    let res = engine.go(&eval, time, depth_limit, node_limit, searchmoves);
                    print!("bestmove {}", res.bestmove);
                    if show_eval {
                        print!(" score {} {}", res.score_type, res.score_value);
                    }
                    if let Some(mv) = res.ponder {
                        print!(" ponder {}", mv);
                    }
                    println!();
                },
                "glm" => {
                    // custom non-uci command
                    let mvs = engine.get_legal_moves();
                    for mv in mvs {
                        print!("{} ", move_transform(mv, engine.get_turn()));
                    }
                    println!();
                },
                "getbb" => {
                    let bbs = engine.get_bitboards();
                    for bb in bbs.iter() {
                        print!("{} ", bb);
                    }
                    println!();
                },
                "eval" => {
                    // custom non-uci command
                    engine.abort.store(false, Ordering::Relaxed);
                    let res =  engine.get_static_eval(&eval);
                    engine.abort.store(true, Ordering::Relaxed);
                    println!("static_score {} is_quiet {} q_score {} qr_score {}", res.score, res.is_quiet, res.q_score, res.qr_score);
                },
                "move" => {
                    // custom non-uci command
                    if cmd.len() < 2 {
                        println!("Error (this argument requires parameter): move");
                        continue;
                    }
                    let res = engine.make_move_safe(cmd[1].to_lowercase().as_str());
                    if !res {
                        println!("Error (illegal move): {}", cmd[1]);
                    }
                },
                "undo" => {
                    // custom non-uci command
                    let res = engine.undo_move_safe();
                    if !res {
                        println!("Error (no moves made): undo")
                    }
                },
                "status" => {
                    println!("{:?}", engine.get_result());
                },
                "quit" | "exit" => {
                    break;
                },
                "export" => {
                    println!("{}", engine.export_fen());
                },
                _ => {

                }
            }
        }
    });

    listen(&tx, abort_listener_clone, ponder_listener_clone);

    let _ = handle.join();
    true
}

/// Read loop
fn listen(tx: &Sender<String>, abort: Arc<AtomicBool>, ponder: Arc<AtomicBool>) {
    loop {
        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(_goes_into_input_above) => {
                
            },
            Err(_no_updates_is_fine) => {
                continue;
            }
        }
        let first = input.split_whitespace().next().unwrap_or("");
        match first.to_lowercase().as_str() {
            "isready" => {
                println!("readyok");
            },
            "ponderhit" => {
                ponder.store(false, Ordering::Relaxed);
            },
            "setoption" | "ucinewgame" | "position" | "go" | "eval" | "glm" | "move" | "undo" | "status" | "export" | "getbb" => {
                abort.store(true, Ordering::Relaxed);
                ponder.store(false, Ordering::Relaxed);
                tx.send(input).unwrap();
            },
            "stop" => {
                abort.store(true, Ordering::Relaxed);
                ponder.store(false, Ordering::Relaxed);
            },
            "quit" | "exit" => {
                abort.store(true, Ordering::Relaxed);
                ponder.store(false, Ordering::Relaxed);
                tx.send(input).unwrap();
                return;
            },
            "uci" => {
                print_greeting();
            },
            _ => {
                println!("Error (unknown command): {}", first);
            }
        }
    }
}

/// faster with pseudo_legal
fn parse_apply_moves(moves: &[&str], engine: &mut Search) {
    for mov in moves {
        engine.make_move(move_transform_back(
            mov,
            &engine.get_pseudo_legal_moves(),
            engine.get_turn()).unwrap()
        );
    }
}

fn print_greeting() {
    println!("id name {}", MYNAME);
    println!("id author {}", AUTHOR);
    println!("option name Hash type spin default 24 min 1 max 24576");
    println!("uciok");
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
        "infinite",
        "eval"
    ])
});