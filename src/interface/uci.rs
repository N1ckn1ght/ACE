use std::{io, sync::mpsc::{Receiver, channel}, thread, time::Duration};
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

            },
            "position" => {

            },
            "go" => {

            },
            "quit" => {
                return;
            }
            _ => {

            }
        }
    }
}