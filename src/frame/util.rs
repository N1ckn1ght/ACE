// The utility file contains A LOT of constants and inline/qol functions to use.
// It's designed to have some dead code in case of necessity and/or testing.

#![allow(dead_code)]

use std::{cmp::min, fs, io::Cursor, path::Path};
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use phf::phf_map;

/* LIMITATIONS */

pub const CACHED_LEAVES_LIMIT: usize = 1000000;
pub const CACHED_BRANCHES_LIMIT: usize = 1000000;
pub const HALF_DEPTH_LIMIT: usize = 64;
pub const NODES_BETWEEN_COMMS: u64 = 0b0001111111111111;

/* SPECIFIED PATHES */

// magic maps
pub const PATH_MR:   &str = "./res/magics_rook";
pub const PATH_BBR:  &str = "./res/blocker_boards_rook";
pub const PATH_AMR:  &str = "./res/attack_maps_rook";
pub const PATH_MB:   &str = "./res/magics_bishop";
pub const PATH_BBB:  &str = "./res/blocker_boards_bishop";
pub const PATH_AMB:  &str = "./res/attack_maps_bishop";
// no attack maps for queen, refer to AMB | AMR after magic operations
// leaping maps
pub const PATH_AMK:  &str = "./res/attack_maps_king";
pub const PATH_AMN:  &str = "./res/attack_maps_knight";
pub const PATH_AMP:  &str = "./res/attack_maps_pawn_white";
pub const PATH_AMP2: &str = "./res/attack_maps_pawn_black";
// secondary maps
pub const PATH_RNK:  &str = "./res/ranks";                              // disincluding current square
pub const PATH_FLS:  &str = "./res/files";
pub const PATH_PPM:  &str = "./res/passing_piece_maps_white";           // all three columns
pub const PATH_PPM2: &str = "./res/passing_piece_maps_black";
pub const PATH_PBM:  &str = "./res/passing_piece_blocked_maps_white";   // only left and right columns
pub const PATH_PBM2: &str = "./res/passing_piece_blocked_maps_black";
pub const PATH_DAMN: &str = "./res/double_attack_maps_knight";          // two move forward for knights (disincluding current square)

/* GLOBAL CONSTANTS (changing them will break everything, starting from STATIC MAPS several blocks below) */

/* Pieces and placements */

pub const E:  usize = 0;
pub const P:  usize = 2;  // white
pub const P2: usize = 3;  // black (white | turn)
pub const N:  usize = 4;
pub const N2: usize = 5;
pub const B:  usize = 6;
pub const B2: usize = 7;
pub const R:  usize = 8;
pub const R2: usize = 9;
pub const Q:  usize = 10;
pub const Q2: usize = 11;
pub const K:  usize = 12;
pub const K2: usize = 13;

pub const RANK_1: u64 = 0x00000000000000FF;
pub const RANK_2: u64 = 0x000000000000FF00;
pub const RANK_3: u64 = 0x0000000000FF0000;
pub const RANK_4: u64 = 0x00000000FF000000;
pub const RANK_5: u64 = 0x000000FF00000000;
pub const RANK_6: u64 = 0x0000FF0000000000;
pub const RANK_7: u64 = 0x00FF000000000000;
pub const RANK_8: u64 = 0xFF00000000000000;
pub const FILE_A: u64 = 0x0101010101010101;
pub const FILE_B: u64 = 0x0202020202020202;
pub const FILE_C: u64 = 0x0404040404040404;
pub const FILE_D: u64 = 0x0808080808080808;
pub const FILE_E: u64 = 0x1010101010101010;
pub const FILE_F: u64 = 0x2020202020202020;
pub const FILE_G: u64 = 0x4040404040404040;
pub const FILE_H: u64 = 0x8080808080808080;

pub const CSMASK: u64 = 0x0000000000000060;
pub const CLMASK: u64 = 0x000000000000000E;

/* Move `special` encoding (change only with corresponding functions below) */

pub const MSE_NOTHING:                 u32 = 0b0000;
pub const MSE_EN_PASSANT:              u32 = 0b1000;
pub const MSE_CASTLE_SHORT:            u32 = 0b0100;
pub const MSE_CASTLE_LONG:             u32 = 0b0010;
pub const MSE_DOUBLE_PAWN:             u32 = 0b0001;
// Note: there's no MSE_PROMOTION, it's encoded by piece, same as CAPTURE and PIECE

pub const MSE_CAPTURE_MIN: u32 = (P as u32) << 28;

/* board.castlings bits
    - it won't correlate with MSE because of the color bits anyway */

pub const CSW: u8 = 0b0001; // castle short white
pub const CSB: u8 = 0b0010; // castle short black
pub const CLW: u8 = 0b0100; // castle long white
pub const CLB: u8 = 0b1000; // castle long black

pub const LARGE: i32 = 0x7FFF;
pub const INF:   i32 = 0xFFFF;

/* Branch cache search flags */

pub const HF_PRECISE: i16 = 1;
pub const HF_LOW: i16 = 2;
pub const HF_HIGH: i16 = 4;

/* INLINE FUNCTIONS (...should they've been implemented using trait/impl?) */

#[inline]
pub fn flip(square: usize) -> usize {       // vertical mirroring, as if board was flipped
    square ^ 56
}

#[inline]
pub fn mirror(square: usize) -> usize {     // diagonal mirroring, as if board was rotated
    square ^ 63
}

/* ! Bitboards ! */

#[inline]
pub fn get_bit(bitboard: u64, bit: usize) -> u64 {
    bitboard & (1 << bit)
}

#[inline]
pub fn set_bit(bitboard: &mut u64, bit: usize) {
    *bitboard |= 1 << bit;
}

#[inline]
pub fn del_bit(bitboard: &mut u64, bit: usize) {
    *bitboard &= !(1 << bit);
}

#[inline]
pub fn gtz(bitboard: u64) -> usize {
    u64::trailing_zeros(bitboard) as usize
}

// used for some weird eval methods
#[inline]
pub fn glz(bitboard: u64) -> usize {
    63 - u64::leading_zeros(bitboard) as usize
}

// return trailing zeros, then remove last bit
#[inline]
pub fn pop_bit(bitboard: &mut u64) -> usize {
    let bit = gtz(*bitboard);
    *bitboard &= *bitboard - 1;
    bit
}

/* 8-bit auxiliaries */

#[inline]
pub fn get_bit8(value: u8, bit: usize) -> u8 {
    value & (1 << bit)
}

/* Move encoding */

// since it's not a struct, let's use more inline fuctions
// let's keep it in Most Valuable Victim - Least Valuable Attacker way
// [4 - SPECIAL][8 - square from][8 - square to][4 - moving piece][4 - promotion][4 - captured piece]
// --- additional note ---
// could be further expanded to 64 bit to store mort sort bits - like, pv, killer moves, etc;
// this will increase the perfomance and simplicity if there's a lot of contributing factors to pre-sort
#[inline]
pub fn move_encode(from: usize, to: usize, piece: usize, capture: usize, promotion: usize, special: u32 ) -> u32 {
    special | (from << 4 | to << 12 | (!piece & 0b1111) << 20 | promotion << 24 | capture << 28) as u32
}

#[inline]
pub fn move_get_from(mov: u32) -> usize {
    (mov >> 4 & 0b11111111) as usize
}

#[inline]
pub fn move_get_from_bb(mov: u32) -> u64 {
    1 << (mov >> 4 & 0b11111111)
}

#[inline]
pub fn move_get_to(mov: u32) -> usize {
    (mov >> 12 & 0b11111111) as usize
}

#[inline]
pub fn move_get_to_bb(mov: u32) -> u64 {
    1 << (mov >> 12 & 0b11111111)
}

#[inline]
pub fn move_get_piece(mov: u32) -> usize {
    (!mov >> 20 & 0b1111) as usize
}

#[inline]
pub fn move_get_promotion(mov: u32) -> usize {
    (mov >> 24 & 0b1111) as usize
}

#[inline]
pub fn move_get_capture(mov: u32) -> usize {
    (mov >> 28 & 0b1111) as usize
}

/* ADDITIONAL DATA STRUCTURES */

#[derive(Copy, Clone)]
pub struct EvalMove {
    pub mov: u32,
    pub score: i32
}

impl EvalMove {
    #[inline]
    pub fn new(mov: u32, score: i32) -> Self {
        EvalMove {
            mov,
            score
        }
    }
}

#[derive(Copy, Clone)]
pub struct EvalBr {
    pub score: i32, 
    pub depth: i16,
    pub flag: i16
}

impl EvalBr {
    #[inline]
    pub fn new(score: i32, depth: i16, flag: i16) -> Self {
        EvalBr {
            score,
            depth,
            flag
        }
    }
}

/* GENERAL FUNCTIONS */

pub fn xor64(mut num: u64) -> u64 {
    num ^= num << 13;
    num ^= num >> 7;
    num ^= num << 17;
    num
}

pub fn init64(f: fn(&mut[u64]), path: &str) {
    if Path::new(path).exists() {
        println!("Found file: {}", path);
    } else {
        println!("Creating file: {}", path);
        let mut vec = vec![0; 64];
        f(&mut vec);
        vector_to_file(&vec, path);
    }
}

/* FILE IO */

// save 1d slice of u64 values to file
pub fn vector_to_file(arr: &[u64], path: &str) {
    let mut buf: Vec<u8> = Vec::with_capacity(arr.len() * 8);
    for elem in arr {
        buf.write_u64::<LittleEndian>(*elem).unwrap();
    }
    match fs::metadata(path) {
        Ok(_) => {
            let path_backup: &str = &(path.to_owned() + ".bkp");
            fs::rename(path, path_backup).expect("Failed to rename an existing file");
            fs::write(path, buf).expect("Failed to write a file");
            fs::remove_file(path_backup).expect("Failed to remove a backup file");
        },
        Err(_) => {
            fs::write(path, buf).expect("Failed to write a file");
        }
    }
}

// load list of u64 values from file as 1d vector
pub fn file_to_vector(path: &str) -> Vec<u64> {
    let buf: Vec<u8> = fs::read(path).expect("Failed to read a file");
    let len = buf.len() / 8;
    let mut cur = Cursor::new(buf);
    let mut vec: Vec<u64> = Vec::with_capacity(len);
    for _ in 0..len {
        vec.push(cur.read_u64::<LittleEndian>().unwrap());
    }
    vec
}

// doesn't require attack index shifts as it's possible to get them on the fly
pub fn magics_to_file(path: &str, magics: &[u64], bits: &[usize], attacks: &[u64]) {
    let mut buf: Vec<u8> = Vec::new();
    let mut shift = 0;
    for i in 0..64 {
        buf.write_u64::<LittleEndian>(magics[i]).unwrap();
        buf.write_u32::<LittleEndian>(bits[i] as u32).unwrap();
        let count = 1 << bits[i];
        for attack in attacks.iter().skip(shift).take(count) {
            buf.write_u64::<LittleEndian>(*attack).unwrap();
        }
        shift += count;
    }   
    match fs::metadata(path) {
        Ok(_) => {
            let path_backup: &str = &(path.to_owned() + ".bkp");
            fs::rename(path, path_backup).expect("Failed to rename an existing file");
            fs::write(path, buf).expect("Failed to write magics to file");
            fs::remove_file(path_backup).expect("Failed to remove a backup file");
        },
        Err(_) => {
            fs::write(path, buf).expect("Failed to write magics to file");
        }
    }
}

// load magics from file, returns 1d attack maps vector AND fills magics, bits, ais arrays
pub fn file_to_magics(path: &str, magics: &mut [u64], bits: &mut [usize], attacks_index_shifts: &mut [usize]) -> Vec<u64> {
    let buf: Vec<u8> = fs::read(path).expect("Failed to read magics from file");
    let mut cur = Cursor::new(buf);
    let mut shift = 0;
    let mut vec: Vec<u64> = Vec::new();
    for i in 0..64 {
        attacks_index_shifts[i] = shift;
        magics[i] = cur.read_u64::<LittleEndian>().unwrap();
        bits[i] = cur.read_u32::<LittleEndian>().unwrap() as usize;
        let count = 1 << bits[i];
        for _ in shift..(shift + count) {
            vec.push(cur.read_u64::<LittleEndian>().unwrap());
        }
        shift += count;
    }
    vec.shrink_to_fit();
    vec
}

/* TESTING PURPOSES */

pub fn visualise(bitboards: &[u64], columns: usize) {
    for i in (0..bitboards.len()).step_by(columns) {
        for j in (0..57).rev().step_by(8) {
            for k in 0..min(bitboards.len() - i, columns) {
                for l in j..j+8 {
                    print!("{}", min(get_bit(bitboards[i + k], l), 1));
                }
                print!("\t");
            }
            println!();
        }
        println!();
    }
}

// since it's slow (and for testing), use 0b10101010... form wherever else possible
pub fn str_to_bb(string: &str) -> u64 {
    u64::from_str_radix(string, 2).expect("Failed to transform string to u64 bitboard")
}

pub fn bb_to_str(bitboard: u64) -> String {
    format!("{bitboard:064b}")
}

pub fn u32_to_str(value: u32) -> String {
    format!("{value:064b}")
}

pub fn usize_to_str(value: usize) -> String {
    format!("{value:064b}")
}

/* STATIC MAPS (testing/print/converting purposes) */

pub static PIECES: phf::Map<char, usize> = phf_map! {
    'p' => P2,
    'P' => P,
    'n' => N2,
    'N' => N,
    'b' => B2,
    'B' => B,
    'r' => R2,
    'R' => R,
    'q' => Q2,
    'Q' => Q,
    'k' => K2,
    'K' => K
};

// TODO (optional): find a way to use usize key, if there is one, same with changing keys to constants from above
pub static PIECES_REV: phf::Map<u32, char> = phf_map! {
    2u32  => 'P',
    3u32  => 'p',
    4u32  => 'N',
    5u32  => 'n',
    6u32  => 'B',
    7u32  => 'b',
    8u32  => 'R',
    9u32  => 'r',
    10u32 => 'Q',
    11u32 => 'q',
    12u32 => 'K',
    13u32 => 'k'
};

/* INTERFACE */

pub fn move_transform(mov: u32) -> String {
    let from = move_get_from(mov);
    let to = move_get_to(mov);
    let promotion = move_get_promotion(mov);
    let mut str = String::new();
    str.push(char::from_u32((from & 7) as u32 + 'a' as u32).unwrap());
    str.push(char::from_u32((from / 8) as u32 + '1' as u32).unwrap());
    str.push(char::from_u32((to   & 7) as u32 + 'a' as u32).unwrap());
    str.push(char::from_u32((to   / 8) as u32 + '1' as u32).unwrap());
    if promotion != E {
        str.push(PIECES_REV[&((promotion | 1) as u32)]);
    }
    str
}

pub fn move_transform_back(input: &str, legal_moves: &[u32]) -> Option<u32> {
    let command     = input.as_bytes();
    let from        = command[0] as usize - 'a' as usize + (command[1] as usize - '0' as usize) * 8 - 8;
    let to          = command[2] as usize - 'a' as usize + (command[3] as usize - '0' as usize) * 8 - 8;
    let mut promo   = E;
    if command.len() > 4 {
        promo = PIECES[&(command[4] as char)] & !1;
    }
    for legal in legal_moves.iter() {
        let mfrom  = move_get_from(*legal);
        let mto    = move_get_to(*legal);
        let mpromo = move_get_promotion(*legal);
        if from == mfrom && to == mto && promo == mpromo {
            return Some(*legal);
        }
    }
    None
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::board::Board;

    #[test]
    fn test_utility_file_io() {
        const PATH: &str = "./TEST_FILE_1";
        let mut arr = [1, 2, 3, 4];
        for _ in 0..20 {
            for j in 0..4 {
                arr[j] = xor64(arr[j]);
            }
        }
        arr[2] = 1;
        vector_to_file(&arr, PATH);
        let arr2 = file_to_vector(PATH);
        fs::remove_file(PATH).expect("Failed to delete a file");
        assert_eq!(arr.len(), arr2.len());
        for i in 0..4 {
            assert_eq!(arr[i], arr2[i]);
        }
    }

    #[test]
    fn test_utility_bb_from_to_str() {
        let a = str_to_bb("0000000000000000000000000000000001001111011011001001001101001111");
        assert_eq!(a, 1332515663);
        let b = bb_to_str(75238572);
        assert_eq!(b, "0000000000000000000000000000000000000100011111000000110010101100");
    }

    #[test]
    fn test_utility_inline_functions() {
        let mut value = 24;
        set_bit(&mut value, 0);
        assert_eq!(get_bit(value, 0), 1);
        assert_eq!(get_bit(value, 1), 0);
        assert_eq!(get_bit(value, 2), 0);
        assert_eq!(get_bit(value, 3), 8);
        assert_eq!(get_bit(value, 4), 16);
        assert_eq!(pop_bit(&mut value), 0);
        assert_eq!(value, 24);
    }

    #[test]
    fn test_utility_move_encoding() {
        let from = 1 << 36;
        let to = 1 << 57;
        let piece = Q2;
        let promotion = P;
        let capture = N;
        let special = MSE_EN_PASSANT;
        let mov = move_encode(gtz(from), gtz(to), piece, capture, promotion, special);
        assert_eq!(move_get_from(mov), gtz(from));
        assert_eq!(move_get_to(mov), gtz(to));
        assert_eq!(move_get_piece(mov), piece);
        assert_eq!(move_get_promotion(mov), promotion);
        assert_eq!(move_get_capture(mov), capture);
        assert_eq!(mov & MSE_CASTLE_SHORT, 0);
        assert_eq!(mov & MSE_CASTLE_LONG, 0);
        assert_eq!(mov & MSE_DOUBLE_PAWN, 0);
        assert_ne!(mov & MSE_EN_PASSANT, 0);
        let mov = move_encode(57, 63, P2, R, Q2, MSE_NOTHING);
        assert_eq!(move_get_promotion(mov), Q2);
        let mov = move_encode(57, 63, P2, R, R2, MSE_NOTHING);
        assert_eq!(move_get_promotion(mov), R2);
        let mov = move_encode(57, 63, P2, R, B2, MSE_NOTHING);
        assert_eq!(move_get_promotion(mov), B2);
        let mov = move_encode(57, 63, P2, R, N2, MSE_NOTHING);
        assert_eq!(move_get_promotion(mov), N2);
    }

    #[test]
    fn test_utility_move_transform_back() {
        let mut board = Board::default();
        let moves = board.get_legal_moves();
        let mov = move_transform_back("e2e4", &moves);
        assert_ne!(mov.is_none(), true);
        let mov = move_transform_back("e2e4q", &moves);
        assert_eq!(mov.is_none(), true);
        let mov = move_transform_back("e2e5", &moves);
        assert_eq!(mov.is_none(), true);
        let mut board = Board::import("r1bq1bnr/ppp1kP1p/2np2p1/4p3/8/3B1N2/PPPP1PPP/RNBQK2R w KQ - 1 7");
        let moves = board.get_legal_moves();
        let mov = move_transform_back("f7g8n", &moves);
        assert_ne!(mov.is_none(), true);
        let mov = move_transform_back("f7f8r", &moves);
        assert_eq!(mov.is_none(), true);
        let mov = move_transform_back("e1g1", &moves);
        assert_ne!(mov.is_none(), true);
    }
}