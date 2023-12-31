mod util;
mod gen;
mod maps;
mod board;

use board::Board;
use gen::{magic::init_magics, leaping::init_leaping_attacks};

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    let mut board = Board::new();

    // TODO: delet this

    let x = board.get_legal_moves();
    board.make_move(0);
    board.revert_move();
    if board.turn {
        println!("yes");
    } else {
        println!("{}", x[0]);
    }
}
