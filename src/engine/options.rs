use std::cmp::{max, min};

pub struct Options {
    rand:       i32,
    // pub memory:  usize
}

impl Default for Options {
    fn default() -> Options {
        Self {
            rand: 20
        }
    } 
}

impl Options {
    #[inline]
    pub fn get_rand(&self) -> i32 {
        self.rand
    }

    pub fn parse(&mut self, query: &str) {
        let cmd = query.split('=').collect::<Vec<&str>>();
        if cmd.is_empty() {
            println!("Error (bad syntax for option): {}", query);
            return;
        }
        match cmd[0] {
            "Random" => {
                let mut rand = cmd[1].parse::<i32>().unwrap_or(5);
                rand = min(max(rand, 1), 50);
                self.rand = rand * 4;
            },
            _ => {
                println!("Error (unknown option): {}", query);
            }
        }
    }
}