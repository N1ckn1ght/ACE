use std::collections::HashMap;

use super::zobrist::Zobrist;

pub struct Opening {
    pub book: HashMap<u64, Vec<WMove>>, // is empty if not initialized
    pub to_init: Vec<Page>              // is empty if initialized
}

impl Default for Opening {
    fn default() -> Opening {
        let mut vec = 
        [
            // Move 1
            Page::new(
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_owned(),
                [
                    Variation::new("g1f3".to_owned(), 4),
                    Variation::new("e2e4".to_owned(), 2),
                    Variation::new("d2d4".to_owned(), 1),
                    Variation::new("b1c3".to_owned(), 1),
                    Variation::new("c2c4".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 2, e4
            Page::new(
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1".to_owned(),
                [
                    Variation::new("e7e5".to_owned(), 16),
                    Variation::new("c7c5".to_owned(), 4),
                    Variation::new("d7d5".to_owned(), 4),
                    Variation::new("b8c6".to_owned(), 4),
                    Variation::new("g8f6".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 2, d4
            Page::new(
                "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1".to_owned(),
                [
                    Variation::new("g8f6".to_owned(), 24),
                    Variation::new("d7d5".to_owned(), 24),
                    Variation::new("f7f5".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 2, c4
            Page::new(
                "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq - 0 1".to_owned(),
                [
                    Variation::new("e7e5".to_owned(), 8),
                    Variation::new("g8f6".to_owned(), 1),
                    Variation::new("c7c5".to_owned(), 1),
                    Variation::new("b8c6".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 2, Nf3
            Page::new(
                "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1".to_owned(),
                [
                    Variation::new("d7d5".to_owned(), 32),
                    Variation::new("g8f6".to_owned(), 16),
                    Variation::new("b8c6".to_owned(), 4),
                    Variation::new("c7c5".to_owned(), 2),
                    Variation::new("e7e6".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 2, Nc3
            Page::new(
                "rnbqkbnr/pppppppp/8/8/8/2N5/PPPPPPPP/R1BQKBNR b KQkq - 1 1".to_owned(),
                [
                    Variation::new("d7d5".to_owned(), 4),
                    Variation::new("g8f6".to_owned(), 2),
                    Variation::new("e7e5".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 3, e4 e5
            Page::new(
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2".to_owned(),
                [
                    Variation::new("g1f3".to_owned(), 8),
                    Variation::new("f1c4".to_owned(), 4),
                    Variation::new("d2d4".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 3, e4 d5
            Page::new(
                "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2".to_owned(),
                [
                    Variation::new("e4d5".to_owned(), 32),
                    Variation::new("d2d4".to_owned(), 8),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 3, e4 Nc6
            Page::new(
                "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2".to_owned(),
                [
                    Variation::new("d2d4".to_owned(), 7),
                    Variation::new("g1f3".to_owned(), 1),
                    Variation::new("b1c3".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 4, e4 e5 Nf3
            Page::new(
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2".to_owned(),
                [
                    Variation::new("b8c6".to_owned(), 32),
                    Variation::new("g8f6".to_owned(), 16),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 4, e4 e5 Bc4
            Page::new(
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2".to_owned(),
                [
                    Variation::new("g8f6".to_owned(), 8),
                    Variation::new("b8c6".to_owned(), 8),
                    Variation::new("f8c5".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 5, e4 e5 Nf3 Nc6
            Page::new(
                "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3".to_owned(),
                [
                    Variation::new("f1c4".to_owned(), 40),
                    Variation::new("d2d4".to_owned(), 24),
                    Variation::new("f1b5".to_owned(), 4),
                    Variation::new("b1c3".to_owned(), 4),
                    Variation::new("a2a4".to_owned(), 1),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 5, e4 e5 Bc4 Nc6
            Page::new(
                "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/8/PPPP1PPP/RNBQK1NR w KQkq - 2 3".to_owned(),
                [
                    Variation::new("g1f3".to_owned(), 48),
                    Variation::new("b1c3".to_owned(), 16),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 6, e4 e5 Nf3 Nc6 Bc4 (any transpose)
            Page::new(
                "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3".to_owned(),
                [
                    Variation::new("g8f6".to_owned(), 32),
                    Variation::new("f8c5".to_owned(), 16),
                    Variation::new("f8f5".to_owned(), 4),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
            // Move 7, e4 e5 Nf3 Nc6 Bc4 Nf6 (any transpose)
            Page::new(
                "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4".to_owned(),
                [
                    Variation::new("f3g5".to_owned(), 16),
                    Variation::new("d2d4".to_owned(), 16),
                    Variation::new("b1c3".to_owned(), 16),
                    Variation::new("".to_owned(), 1)
                ].to_vec()
            ),
        ];

        Self {
            book: HashMap::default(),
            to_init: vec.to_vec()
        }
    }
}

impl Opening {
    fn init(&mut self, zob: &Zobrist) {
        for page in self.to_init.iter() {
            
        }
    }
    
    fn is_initialized(&self) -> bool {
        self.to_init.is_empty() && !self.book.is_empty()
    }
}

#[derive(Copy, Clone)]
pub struct WMove {
    pub mov: u32,
    pub chance: u32
}

impl WMove {
    pub fn new(mov: u32, chance: u32) -> Self {
        Self {
            mov,
            chance
        }
    }
}

#[derive(Clone)]
struct Page {
    pub position: String,
    pub variations: Vec<Variation>
}

impl Page {
    fn new(position: String, variations: Vec<Variation>) -> Self {
        Self {
            position,
            variations
        }
    }
}

#[derive(Clone)]
struct Variation {
    pub raw: String,
    pub chance: u32
}

impl Variation {
    fn new(raw: String, chance: u32) -> Self {
        Self {
            raw,
            chance
        }
    }
}