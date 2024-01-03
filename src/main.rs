mod util;
mod gen;
mod maps;
mod board;

use crate::board::Board;
use crate::gen::{magic::init_magics, leaping::init_leaping_attacks};
use crate::util::move_transform;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    let mut board = Board::new();

    let moves = board.get_legal_moves();
    for (tabs, mov) in moves.into_iter().enumerate() {
        if tabs % 5 == 0 {
            println!();
        }
        print!("{}\t", move_transform(mov));
    }
}
