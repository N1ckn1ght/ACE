mod util;
mod gen;
mod maps;
mod board;

use crate::board::Board;
use crate::gen::{magic::init_magics, leaping::init_leaping_attacks};
use crate::util::*;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();

    /* Used for testing, remove later */

    let x = "2kr3r/p1pqnp1p/2nb4/1p1p1b2/3PpPpN/N2QB1P1/PPP1P1BP/R3K2R b KQ - 1 13";
    let mut board = Board::import(&x);

    println!("{}\t{}\t{}", "----", "--", x);
    let moves = board.get_legal_moves();
    for mov in moves.iter() {
        board.make_move(*mov);
        let x = board.get_legal_moves();
        println!("{}\t{}\t{}\t{}", *mov, move_transform(*mov), x.len(), board.export());
        board.revert_move();
    }

    let moves = board.get_legal_moves();
    for (tabs, mov) in moves.into_iter().enumerate() {
        if tabs % 5 == 0 {
            println!();
        }
        print!("{}\t", move_transform(mov));
    }
    println!("\n");
    visualise(&board.bbs, 12);
}
