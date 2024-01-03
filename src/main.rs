mod util;
mod gen;
mod maps;
mod board;

use crate::board::Board;
use crate::gen::{magic::init_magics, leaping::init_leaping_attacks};
use crate::util::{move_transform, visualise};

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    let mut board = Board::import("8/8/8/2k5/2pP4/8/B7/4K3 b - d3 0 3");

    let moves = board.get_legal_moves();
    for (tabs, mov) in moves.into_iter().enumerate() {
        if tabs % 5 == 0 {
            println!();
        }
        print!("{}\t", move_transform(mov));
    }
    println!("\n");
    visualise(&board.bbs, 12);
    println!("{}", board.en_passant);
}
