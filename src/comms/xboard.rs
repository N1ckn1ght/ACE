use std::io;
use crate::{engine::chara::Chara};

static STATE: i8 = 0;
    
pub fn read(chara: &mut Chara) {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_goes_into_input_above) => {
            input.trim().to_string();
            match input.as_str() {
                "xboard" => {
                    
                },
                "new" => {

                },
                "quit" => {

                },
                _ => {}
            }
        },
        Err(_no_updates_is_fine) => {
            return;
        },
    }
}