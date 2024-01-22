// This module allows search for magics,
// it will save results to specified PATHes (utility.rs).

// Magics are numbers that are necessary for
// hashing attack maps for sliding pieces (R, B, Q),
// the only reliant way to find them is a bruteforce.

// The algorithm of feeding in randoms provided by Tord Romstad
// www.talkchess.com/forum3/viewtopic.php?topic_view=threads&p=175834

use std::path::Path;
use crate::util::*;

pub fn init_magics(seed: &mut u64) {
    let mut blocker_boards_rook  : Vec<u64>;
    let mut blocker_boards_bishop: Vec<u64>;

    // init blocker boards for rook if not present

    if Path::new(PATH_BBR).exists() {
        println!("Found rook blocker boards.");
        blocker_boards_rook = file_to_vector(PATH_BBR);
    } else {
        println!("No rook blocker boards found! Creating file at: {}", PATH_BBR);
        blocker_boards_rook = vec![0; 64];
        init_blocker_boards(true, &mut blocker_boards_rook);
        vector_to_file(&blocker_boards_rook, PATH_BBR);
    }

    // search for magic & generate attack maps if not present

    if Path::new(PATH_AMR).exists() {
        println!("Rook attack maps are present.");
    } else {
        println!("No rook attack maps found! Initiating search for magic...");
        let mut magics_rook          : Vec<u64>;
        let comb_bits_rook           : Vec<usize> = Vec::from([
            12, 11, 11, 11, 11, 11, 11, 12,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            12, 11, 11, 11, 11, 11, 11, 12
        ]);
        // note: in some cases it's possible to go further below this bit count,
        // search of such magic will require a lot of time
        let magic_bits_rook = &comb_bits_rook;

        // search for magic

        if Path::new(PATH_MR).exists() {
            println!("Found rook magics.");
            magics_rook = file_to_vector(PATH_MR);
        } else {
            println!("No rook magics found! Creating file at: {}", PATH_MR);
            magics_rook = vec![0; 64];
            let mut magic = *seed;
            for i in 0..64 {
                magics_rook[i] = search_for_magic(i, true, comb_bits_rook[i], magic_bits_rook[i], blocker_boards_rook[i], 1 << 20, seed, &mut magic);
            }
        }

        // generate attack maps

        let mut total_capacity = 0;
        for bits in magic_bits_rook.iter() {
            total_capacity += 1 << bits;
        }
        let mut maps = vec![0; total_capacity];
        let mut current_capacity = 0;
        for i in 0..64 {
            let mut combs = vec![0; 1 << comb_bits_rook[i]];
            init_combs(&mut combs, blocker_boards_rook[i]);
            let mut attacks = vec![0; 1 << magic_bits_rook[i]];
            init_attacks(i, true, &mut attacks, &mut combs, 1 << magic_bits_rook[i]);
            for (j, comb) in combs.iter().enumerate() {
                let magic_index = (comb.wrapping_mul(magics_rook[i]) >> (64 - magic_bits_rook[i])) as usize;
                maps[magic_index + current_capacity] = attacks[j];
            }
            current_capacity += 1 << magic_bits_rook[i];
        }
        magics_to_file(PATH_AMR, &magics_rook, magic_bits_rook, &maps);
    }

    // init blocker boards for bishop if not present
    
    if Path::new(PATH_BBB).exists() {
        println!("Found bishop blocker boards.");
        blocker_boards_bishop = file_to_vector(PATH_BBB);
    } else {
        println!("No bishop blocker boards found! Creating file at: {}", PATH_BBR);
        blocker_boards_bishop = vec![0; 64];
        init_blocker_boards(false, &mut blocker_boards_bishop);
        vector_to_file(&blocker_boards_bishop, PATH_BBB);
    }

    // search for magic & generate attack maps if not present

    if Path::new(PATH_AMB).exists() {
        println!("Bishop attack maps are present.");
    } else {
        println!("No bishop attack maps found! Initiating search for magic...");

        let mut magics_bishop        : Vec<u64>;
        let comb_bits_bishop         : Vec<usize> = Vec::from([
            6, 5, 5, 5, 5, 5, 5, 6,
            5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 7, 7, 7, 7, 5, 5,
            5, 5, 7, 9, 9, 7, 5, 5,
            5, 5, 7, 9, 9, 7, 5, 5,
            5, 5, 7, 7, 7, 7, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5,
            6, 5, 5, 5, 5, 5, 5, 6
        ]);
        let magic_bits_bishop = &comb_bits_bishop;
    
        // search for magic

        if Path::new(PATH_MB).exists() {
            println!("Found bishop magics.");
            magics_bishop = file_to_vector(PATH_MB);
        } else {
            println!("No bishop magics found! Creating file at: {}", PATH_MB);
            magics_bishop = vec![0; 64];
            let mut magic = *seed;
            for i in 0..64 {
                magics_bishop[i] = search_for_magic(i, false, comb_bits_bishop[i], magic_bits_bishop[i], blocker_boards_bishop[i], 1 << 18, seed, &mut magic);
            }
        }

        // generate attack maps

        let mut total_capacity = 0;
        for bits in magic_bits_bishop.iter() {
            total_capacity += 1 << bits;
        }
        let mut maps = vec![0; total_capacity];
        let mut current_capacity = 0;
        for i in 0..64 {
            let mut combs = vec![0; 1 << comb_bits_bishop[i]];
            init_combs(&mut combs, blocker_boards_bishop[i]);
            let mut attacks = vec![0; 1 << magic_bits_bishop[i]];
            init_attacks(i, false, &mut attacks, &mut combs, 1 << magic_bits_bishop[i]);
            for (j, comb) in combs.iter().enumerate() {
                let magic_index = (comb.wrapping_mul(magics_bishop[i]) >> (64 - magic_bits_bishop[i])) as usize;
                maps[magic_index + current_capacity] = attacks[j];
            }
            current_capacity += 1 << magic_bits_bishop[i];
        }
        magics_to_file(PATH_AMB, &magics_bishop, magic_bits_bishop, &maps);
    }
}

#[allow(clippy::too_many_arguments)] // c'mon, this is the way!
fn search_for_magic(sq: usize, is_rook: bool, bits: usize, target: usize, bb: u64, limit: usize, seed: &mut u64, magic: &mut u64) -> u64 {
    let mut text = "BISHOP";
    if is_rook {
        text = "ROOK";
    }

    let comb_count: usize = 1 << bits;
    let hash_count: usize = 1 << target;
    
    // generate all possible combinations of blockers (we still don't care about last ranks and files)
    let mut combs: Vec<u64> = vec![0; comb_count];
    init_combs(&mut combs, bb);
    
    // generate attack maps for every combination of blockers (there WE CARE about last ranks and files!)
    let mut attacks: Vec<u64> = vec![0; comb_count];
    init_attacks(sq, is_rook, &mut attacks, &mut combs, comb_count);

    let mut fail = true;
    for _tries in 0..limit {
        // fast heuristic optimization
        if u64::count_ones(bb.wrapping_mul(*magic) & 0xFF00000000000000) < 6 {
            *magic = next_random_magic(&mut *seed);
            continue;
        }

        fail = false;
        let mut used = vec![0; hash_count];
        for i in 0..comb_count {
            let index = (combs[i].wrapping_mul(*magic) >> (64 - target)) as usize;
            // different blocker boards often will produce same attack maps (e.g. 0-1-square-..., 1-1-square-... => 0-1-square-...)
            // it's totally ok to have hash collision in such scenario; although otherwise, the magic has failed.
            if used[index] > 0 && used[index] != attacks[i] {
                fail = true;
                break;
            }
            used[index] = attacks[i];
        }

        if fail {
            *magic = next_random_magic(&mut *seed);
        } else {
            break;
        }
    }

    // probably shouldn't even panic, but with proper seed and decent target bits count, this will never occur anyway
    if fail {
        panic!("Failed to generate a magic!\n{} {}, {} bits, tries = {}", text, sq, target, limit);
    } else {
        println!("{} {}, {} bits, new magic = {}", text, sq, target, magic);
    }
    *magic
}

// similar to attack in empty blocker board comb, but we stop before the last rank/file
/* Examples:

    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 1 0 0 0 1 0 0    0 0 0 0 0 0 1 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 0 1 0 1 0 0 0    0 0 0 0 0 1 0 0
    0 1 1 T 1 1 1 0    1 0 0 0 0 0 0 0    0 0 0 T 0 0 0 0    0 0 0 0 1 0 0 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 0 1 0 1 0 0 0    0 0 0 1 0 0 0 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 1 0 0 0 1 0 0    0 0 1 0 0 0 0 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 0 0 0 0 0 1 0    0 T 0 0 0 0 0 0
    0 0 0 0 0 0 0 0    T 1 1 1 1 1 1 0    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0
*/
fn init_blocker_boards(is_rook: bool, bbs: &mut [u64]) {
    if is_rook {
        for (i, bb) in bbs.iter_mut().enumerate() {
            // bbs[i] = 0;
            for j in (i..56).step_by(8) {                                       // up
                set_bit(bb, j);
            }
            for j in (8..i + 1).rev().step_by(8) {                              // down (including 8, i)
                set_bit(bb, j);
            }
            for j in (i & 56) + 1..i + 1 {                                     // left (still left-to-right though)
                set_bit(bb, j);
            }
            for j in i..(i | 7) {                                   // right
                set_bit(bb, j);
            }
            del_bit(bb, i);
        }
    } else {
        for (i, bb) in bbs.iter_mut().enumerate() {
            // bbs[i] = 0;
            for j in (i..).step_by(7).take_while(|j| j / 8 < 7 && j & 7 > 0) {  // up-left
                set_bit(bb, j);
            }
            for j in (i..).step_by(9).take_while(|j| j / 8 < 7 && j & 7 < 7) {  // up-right
                set_bit(bb, j);
            }
            // if you'll find a way to do this using for loops, leave an issue on github =)
            let mut j = i;
            while j / 8 > 0 && j & 7 < 7 {                                      // down-right
                set_bit(bb, j);
                j -= 7;
            }
            j = i;
            while j / 8 > 0 && j & 7 > 0 {                                      // down-left
                set_bit(bb, j);
                j -= 9;
            }
            del_bit(bb, i);
        }
    }
}

// init all possible permutations of blockers by given blocker board
fn init_combs(combs: &mut [u64], bb: u64) {
    for (i, comb) in combs.iter_mut().enumerate() {
        let mut mask = bb;
        let mut bit: usize = 0;
        while mask > 0 {
            let csq = pop_bit(&mut mask);
            if i & (1 << bit) > 0 {
                set_bit(comb, csq);
            }
            bit += 1;
        }
    }
}

/* Example of an attack result by given comb for a rook:
    (we can always do 'ATTACKS &= !ALLY_PIECES' stuff later in the board/engine logic)

    0 0 0 1 0 0 0 0    0 0 0 1 0 0 0 0          *the '1' at the upper rank is not possible
    0 0 0 0 0 0 0 0    0 0 0 1 0 0 0 0           because of the blocker board generation,
    0 0 0 0 0 0 0 0    0 0 0 1 0 0 0 0           but this example explains why
    0 1 0 T 0 0 0 0    0 1 1 T 1 1 1 1           it's designed such way
    0 0 0 1 0 0 0 0    0 0 0 1 0 0 0 0
    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0
    0 0 0 1 0 0 0 0    0 0 0 0 0 0 0 0 
    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0
*/
fn init_attacks(sq: usize, is_rook: bool, attacks: &mut [u64], combs: &mut [u64], count: usize) {
    for i in 0..count {
        // attacks[i] = 0;
        if is_rook {
            for j in (sq..64).step_by(8) {              // up
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) > 0 {
                    break;
                }
            }
            for j in (0..sq + 1).rev().step_by(8) {     // down
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) > 0 {
                    break;
                }
            }
            for j in (sq & 56..sq + 1).rev() {          // left
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) > 0 {
                    break;
                }
            }
            for j in sq..(sq | 7) + 1 {                 // right
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) > 0 {
                    break;
                }
            }
            del_bit(&mut attacks[i], sq);
        } else {
            let mut j = sq;
            loop {                                      // up-right
                j += 7;
                if j & 7 >= sq & 7 || j > 62 {
                    break;
                }
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) > 0 {
                    break;
                }
            }
            j = sq;
            loop {                                      // up-left
                j += 9;
                if j & 7 <= sq & 7 || j > 63 {
                    break;
                }
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) > 0 {
                    break;
                }
            }
            j = sq;
            loop {                                      // down-left
                if j < 8 {
                    break;
                }
                j -= 7;
                if j & 7 <= sq & 7 {
                    break;
                }
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) > 0 {
                    break;
                }
            }
            j = sq;
            loop {                                      // down-right
                if j < 9 {
                    break;
                }
                j -= 9;
                if j & 7 >= sq & 7 {
                    break;
                }
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) > 0 {
                    break;
                }
            }
        }
    }
}

fn next_random_magic(seed: &mut u64) -> u64 {
    *seed = xor64(*seed);
    let mut magic = *seed;
    *seed = xor64(*seed);
    magic &= *seed;
    *seed = xor64(*seed);
    magic &= *seed;
    magic
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::{bb_to_str, str_to_bb};

    #[test]
    fn test_magic_blockers() {
        let mut bbsr = vec![0; 64];
        let mut bbsb = vec![0; 64];
        init_blocker_boards(true, &mut bbsr);
        init_blocker_boards(false, &mut bbsb);

        assert_eq!("0000000001000000001000000001000000001000000001000000001000000000", bb_to_str(bbsb[0]));
        assert_eq!("0000000000000000000000000000000001000000001000100001010000000000", bb_to_str(bbsb[3]));
        assert_eq!("0000000001010000000000000101000000001000000001000000001000000000", bb_to_str(bbsb[45]));
        assert_eq!("0000000000100001000100010000100100000101000000110111111000000000", bb_to_str(bbsb[8]  | bbsr[8]));
        assert_eq!("0000000000000010000000100000001000000010000000100111110000000000", bb_to_str(bbsr[9]));
        assert_eq!("0111011000011100001010100100100000001000000010000000100000000000", bb_to_str(bbsb[59] | bbsr[59]));
        assert_eq!("0000000001010100001110000110111000111000010101000001001000000000", bb_to_str(bbsb[36] | bbsr[36]));
        assert_eq!("0000000001000000001000000001000000001000000001000000001000000000", bb_to_str(bbsb[63]));
        assert_eq!("0000000000000010000001000000100000010000001000000100000000000000", bb_to_str(bbsb[56]));
    }

    #[test]
    fn test_magic_combs() {
        // bbsr[0]
        let mut rook_bb = str_to_bb("0000000010000000100000001000000010000000100000001000000001111110");
        let rook_bits = 12;
        let mut rook_combs = vec![0; 1 << rook_bits];
        init_combs(&mut rook_combs, rook_bb);
        
        // bbsb[59]
        let bishop_bb = str_to_bb("0000000000010100001000100100000000000000000000000000000000000000");
        let bishop_bits = 5;
        let mut bishop_combs = vec![0; 1 << bishop_bits];
        init_combs(&mut bishop_combs, bishop_bb);

        assert_eq!(0, rook_combs[0]);
        assert_eq!(rook_bb, *rook_combs.last().unwrap());
        assert_eq!(0, bishop_combs[0]);
        assert_eq!(bishop_bb, *bishop_combs.last().unwrap());
        
        pop_bit(&mut rook_bb);
        assert_eq!(rook_bb, rook_combs[(1 << rook_bits) - 2]);
    }

    #[test]
    fn test_magic_attacks() {
        let mut combr = vec![str_to_bb("0000000000000000000001000011000000000100000000000000010000000000")];
        let mut attackr = vec![0];
        init_attacks(34, true, &mut attackr, &mut combr, 1);
        assert_eq!("0000000000000000000001000001101100000100000000000000000000000000", bb_to_str(attackr[0]));
    
        let mut combb = vec![str_to_bb("0000000000100000000001000000000000000000001000000100000000000000")];
        let mut attackb = vec![0];
        init_attacks(35, false, &mut attackb, &mut combb, 1);
        assert_eq!("0000000000100000000101000000000000010100001000100000000100000000", bb_to_str(attackb[0]));

        let mut combb = vec![str_to_bb("0000000000001000000000000000000000000000000000000000010000000000")];
        let mut attackb = vec![0];
        init_attacks(37, false, &mut attackb, &mut combb, 1);
        assert_eq!("0000000010001000010100000000000001010000100010000000010000000000", bb_to_str(attackb[0]));

        attackb[0] = 0;
        init_attacks(63, false, &mut attackb, &mut combb, 1);
        assert_eq!("0000000001000000001000000001000000001000000001000000001000000001", bb_to_str(attackb[0]));

        attackb[0] = 0;
        init_attacks(56, false, &mut attackb, &mut combb, 1);
        assert_eq!("0000000000000010000001000000100000010000001000000100000010000000", bb_to_str(attackb[0]));

        attackb[0] = 0;
        init_attacks( 8, false, &mut attackb, &mut combb, 1);
        assert_eq!("0100000000100000000100000000100000000100000000100000000000000010", bb_to_str(attackb[0]));

        attackb[0] = 0;
        init_attacks( 7, false, &mut attackb, &mut combb, 1);
        assert_eq!("0000000100000010000001000000100000010000001000000100000000000000", bb_to_str(attackb[0]));

        attackb[0] = 0;
        init_attacks(57, false, &mut attackb, &mut combb, 1);
        assert_eq!("0000000000000101000010000001000000100000010000001000000000000000", bb_to_str(attackb[0]));
    }

    #[test]
    fn test_magic_next_random() {
        let mut seed = 1050;
        let s1 = seed;
        let magic = next_random_magic(&mut seed);
        assert_ne!(seed, s1);
        assert_ne!(seed, magic);
        assert_ne!(s1, magic);
        assert_ne!(magic, 0);
    }
}