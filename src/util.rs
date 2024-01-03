// The utility file contains A LOT of constants and inline/qol functions to use.
// It's designed to have some dead code in case of necessity and/or testing.

#![allow(dead_code)]

use std::{cmp::min, fs, io::Cursor};
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use phf::phf_map;

/* SPECIFIED PATHES */

pub static PATH_MR:   &str = "./res/magics_rook";
pub static PATH_BBR:  &str = "./res/blocker_boards_rook";
pub static PATH_AMR:  &str = "./res/attack_maps_rook";
pub static PATH_MB:   &str = "./res/magics_bishop";
pub static PATH_BBB:  &str = "./res/blocker_boards_bishop";
pub static PATH_AMB:  &str = "./res/attack_maps_bishop";
pub static PATH_AMK:  &str = "./res/attack_maps_king";
pub static PATH_AMN:  &str = "./res/attack_maps_knight";
pub static PATH_AMP:  &str = "./res/attack_maps_pawn_white";
pub static PATH_AMP2: &str = "./res/attack_maps_pawn_black";
// no attack maps for queen, refer to AMB | AMR after magic operations

/* GLOBAL CONSTANTS (changing them will break everything, starting from STATIC MAPS several blocks below) */

/* Pieces and placements */

pub static P:  usize = 0;  // white
pub static P2: usize = 1;  // black (white | turn)
pub static N:  usize = 2;
pub static N2: usize = 3;
pub static B:  usize = 4;
pub static B2: usize = 5;
pub static R:  usize = 6;
pub static R2: usize = 7;
pub static Q:  usize = 8;
pub static Q2: usize = 9;
pub static K:  usize = 10;
pub static K2: usize = 11;
pub static E:  usize = 12; // no piece (0b1100)

pub static RANK_1: u64 = 0x00000000000000FF;
pub static RANK_2: u64 = 0x000000000000FF00;
pub static RANK_3: u64 = 0x0000000000FF0000;
pub static RANK_4: u64 = 0x00000000FF000000;
pub static RANK_5: u64 = 0x000000FF00000000;
pub static RANK_6: u64 = 0x0000FF0000000000;
pub static RANK_7: u64 = 0x00FF000000000000;
pub static RANK_8: u64 = 0xFF00000000000000;
pub static FILE_A: u64 = 0x0101010101010101;
pub static FILE_B: u64 = 0x0202020202020202;
pub static FILE_C: u64 = 0x0404040404040404;
pub static FILE_D: u64 = 0x0808080808080808;
pub static FILE_E: u64 = 0x1010101010101010;
pub static FILE_F: u64 = 0x2020202020202020;
pub static FILE_G: u64 = 0x4040404040404040;
pub static FILE_H: u64 = 0x8080808080808080;

pub static CSMASK: u64 = 0x0000000000000060;
pub static CLMASK: u64 = 0x000000000000000E;

/* Move `special` encoding (change only with corresponding functions below)
    - now with u64 any can use MSE_CHECK | MSE_EN_PASSANT by bitwise OR instead of ciphering functions */

pub static MSE_NOTHING:                 u64 = 0b00000000;
pub static MSE_DOUBLE_CHECK:            u64 = 0b10000000;
pub static MSE_CHECK:                   u64 = 0b01000000;
pub static MSE_EN_PASSANT:              u64 = 0b00001000;
pub static MSE_CASTLE_SHORT:            u64 = 0b00000100;
pub static MSE_CASTLE_LONG:             u64 = 0b00000010;
pub static MSE_DOUBLE_PAWN:             u64 = 0b00000001;
// Note: there's no MSE_PROMOTION, it's in a different encoding section

/* board.castlings bits
    - it won't correlate with MSE because of the color bits anyway */

pub static CSW: u8 = 0b0001; // castle short white
pub static CSB: u8 = 0b0010; // castle short black
pub static CLW: u8 = 0b0100; // castle long white
pub static CLB: u8 = 0b1000; // castle long black

/* INLINE FUNCTIONS (...should they've been implemented using trait/impl?) */

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
// since minimum piece (P) is 0, empty piece will be encoded to E (or any other value greater than K2, e.g. 12)
// also this time it's not necessary to have a certain encoding struct to sort by (because of iterative deepening)
// I dislike using u64, but in the end it allows to store everything without heavy ciphering/deciphering
// [8 - SPECIAL][8 - square from][8 - square to][4 - moving piece][4 - promotion][4 - captured piece][28 - FREE]
#[inline]
pub fn move_encode(from: usize, to: usize, piece: usize, capture: usize, promotion: usize, special: u64 ) -> u64 {
    // x86 systems are not gonna like that
    special | (from << 8 | to << 16 | piece << 24 | promotion << 28) as u64 | (capture as u64) << 32
}

#[inline]
pub fn move_get_from(mov: u64) -> usize {
    (mov >> 8 & 0b11111111) as usize
}

#[inline]
pub fn move_get_from_bb(mov: u64) -> u64 {
    1 << (mov >> 8 & 0b11111111)
}

#[inline]
pub fn move_get_to(mov: u64) -> usize {
    (mov >> 16 & 0b11111111) as usize
}

#[inline]
pub fn move_get_to_bb(mov: u64) -> u64 {
    1 << (mov >> 16 & 0b11111111)
}

#[inline]
pub fn move_get_piece(mov: u64) -> usize {
    (mov >> 24 & 0b1111) as usize
}

#[inline]
pub fn move_get_promotion(mov: u64) -> usize {
    (mov >> 28 & 0b1111) as usize
}

#[inline]
pub fn move_get_capture(mov: u64) -> usize {
    (mov >> 32 & 0b1111) as usize
}

/* GENERAL FUNCTIONS */

pub fn xor64(mut num: u64) -> u64 {
    num ^= num << 13;
    num ^= num >> 7;
    num ^= num << 17;
    num
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
    0u32  => 'P',
    1u32  => 'p',
    2u32  => 'N',
    3u32  => 'n',
    4u32  => 'B',
    5u32  => 'b',
    6u32  => 'R',
    7u32  => 'r',
    8u32  => 'Q',
    9u32  => 'q',
    10u32 => 'K',
    11u32 => 'k'
};

/* INTERFACE */

pub fn move_transform(mov: u64) -> String {
    let from = move_get_from(mov);
    let to = move_get_to(mov);
    let promotion = move_get_promotion(mov);
    let mut str = String::new();
    str.push(char::from_u32((from % 8) as u32 + 'a' as u32).unwrap());
    str.push(char::from_u32((from / 8) as u32 + '1' as u32).unwrap());
    str.push(char::from_u32((to % 8) as u32 + 'a' as u32).unwrap());
    str.push(char::from_u32((to / 8) as u32 + '1' as u32).unwrap());
    if promotion < E {
        str.push(PIECES_REV[&((promotion | 1) as u32)]);
    }
    str
}



#[cfg(test)]
mod tests {
    use super::*;

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
        let special = MSE_DOUBLE_CHECK | MSE_EN_PASSANT;
        // println!("{}\n{}\n{}\n{}\n{}\n{}", bb_to_str(special), usize_to_str(gtz(from) << 8), usize_to_str(gtz(to) << 16), usize_to_str(piece << 24), usize_to_str(promotion << 28), usize_to_str(capture << 32));
        let mov = move_encode(gtz(from), gtz(to), piece, capture, promotion, special);
        // println!("{}", bb_to_str(mov));
        assert_eq!(move_get_from(mov), gtz(from));
        assert_eq!(move_get_to(mov), gtz(to));
        assert_eq!(move_get_piece(mov), piece);
        assert_eq!(move_get_promotion(mov), promotion);
        assert_eq!(move_get_capture(mov), capture);
        assert_eq!(mov & MSE_CASTLE_SHORT, 0);
        assert_eq!(mov & MSE_CASTLE_LONG, 0);
        assert_eq!(mov & MSE_DOUBLE_PAWN, 0);
        assert_eq!(mov & MSE_CHECK, 0);
        assert_ne!(mov & MSE_EN_PASSANT, 0);
        assert_ne!(mov & MSE_DOUBLE_CHECK, 0);
    }
}