use std::cmp::{max, min};

pub struct Options {
    pub rand:        i32,
    pub rand_status: bool
    // pub memory:  usize
}

impl Default for Options {
    fn default() -> Options {
        Self {
            rand: 20,
            rand_status: false
        }
    } 
}

impl Options {
    pub fn parse(&mut self, query: &str) {
        let cmd = query.split('=').collect::<Vec<&str>>();
        if cmd.is_empty() {
            println!("Error (bad syntax for option): {}", query);
            return;
        }
        match cmd[0] {
            "Random" => {
                let mut rand = cmd[1].parse::<i32>().unwrap_or(5);
                rand = min(max(rand, 0), 50);
                self.rand = rand * 4;
            },
            _ => {
                println!("Error (unknown option): {}", query);
            }
        }
    }
}