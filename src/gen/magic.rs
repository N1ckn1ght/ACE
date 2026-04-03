use crate::frame::util::*;


/// Search for magic numbers for perfect hashing of board blockers to calculate sliding pieces attacks
/// 
/// The algorithm of feeding in randoms provided by Tord Romstad
/// www.talkchess.com/forum3/viewtopic.php?topic_view=threads&p=175834
fn search_for_magic(
    sq: usize,
    is_rook: bool,
    target: usize,
    bb: u64,
    seed: &mut u64
) -> u64 {
    // generate all possible combinations of blockers (we still don't care about last ranks and files)
    let combs = get_combs(bb);

    // generate attack maps for every combination of blockers (here WE DO CARE about last ranks and files)
    let attacks = get_attacks(sq, is_rook, &combs);

    // bruteforce approach, let's go over random numbers and find magic ones
    let mut magic= 0;
    let mut attempt = 0;
    loop {
        attempt += 1;
        if attempt & 0b111111111111111111111 == 0 {
            log(&format!("warn? attempt {} on sq {} is_rook {} target {} seed {} last try {}", attempt, sq, is_rook, target, seed, magic));
        }

        magic = next_random_magic(&mut *seed);

        // fast heuristic optimization
        if u64::count_ones(bb.wrapping_mul(magic) & 0xFF00000000000000) < 6 {
            continue;
        }

        // we try to hash positions and see if there's a collision* or not
        // hashed value should contain necessary information about blockers
        // we are not interested in other pieces placements
        if test_magic(magic, target, &combs, &attacks) {
            break;
        }
    }

    magic
}

fn test_magic(magic: u64, target: usize, combs: &Vec<u64>, attacks: &Vec<u64>) -> bool {
    let mut success = true;
    let mut used = vec![0; attacks.len()];
    for i in 0..combs.len() {
        let index = (combs[i].wrapping_mul(magic) >> (64 - target)) as usize;
        // *except different blocker boards often produce the same attack map
        // and the same hash on the same attack is even better!
        if used[index] != 0 && used[index] != attacks[i] {
            success = false;
            break;
        }
        used[index] = attacks[i];
    }
    success
}

/// Generate and return attack maps array (1d), magics themself, magic bit counts, and an array of shifts to get an attack for the square from attack map array
pub fn get_magic_maps(
    is_rook: bool,
    magic_numbers: Option<&Vec<u64>>,
    blocker_boards: Option<&Vec<u64>>,
    seed: Option<u64>
) -> (
    Vec<u64>,    // attack maps (1d array, all results combined)
    Vec<u64>,    // magic numbers
    Vec<usize>,  // magic bit counts
    Vec<usize>,  // attack index shifts
) {
    let comb_bits: Vec<usize> = if is_rook {
        Vec::from([
            12, 11, 11, 11, 11, 11, 11, 12,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            11, 10, 10, 10, 10, 10, 10, 11,
            12, 11, 11, 11, 11, 11, 11, 12
        ])
    } else {
        Vec::from([
            6, 5, 5, 5, 5, 5, 5, 6,
            5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 7, 7, 7, 7, 5, 5,
            5, 5, 7, 9, 9, 7, 5, 5,
            5, 5, 7, 9, 9, 7, 5, 5,
            5, 5, 7, 7, 7, 7, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5,
            6, 5, 5, 5, 5, 5, 5, 6
        ])
    };

    let bbs = match blocker_boards {
        Some(boards) => {
            boards.clone()
        },
        None => {
            get_blocker_boards(is_rook)
        }
    };

    // it's possible to go further below this bit count, but search of such magic is consuming
    let magic_bits = &comb_bits;

    // call search for magic or use presented values
    let magics = match magic_numbers {
        Some(numbers) => {
            log(&format!("Magic numbers (is_rook = {}) are available, testing...", is_rook));
            let bbs = get_blocker_boards(is_rook);
            for (i, magic) in numbers.iter().enumerate() {
                let combs = get_combs(bbs[i]);
                let attacks = get_attacks(i, is_rook, &combs);
                if !test_magic(*magic, magic_bits[i], &combs, &attacks) {
                    panic!("Incorrect magic numbers were passed! See \"src/frame/maps.rs\" and consider generating new ones.");
                }
            }
            log(&format!("OK"));
            numbers.clone()
        },
        None => {
            log(&format!("Magic numbers (is_rook = {}) are not found, generating...", is_rook));
            let mut numbers = vec![0; 64];
            let mut seed = match seed {
                Some(seed) => {
                    seed
                },
                None => {
                    12345679  // fallback seed, paste any
                }
            };
            for i in 0..64 {
                numbers[i] = search_for_magic(i, is_rook, magic_bits[i], bbs[i], &mut seed);
                log(&format!("Found magic | {:2} sq: {}", i, numbers[i]));
            }
            numbers
        }
    };

    // generate attack maps
    let mut total_capacity = 0;
    for bits in magic_bits.iter() {
        total_capacity += 1 << bits;
    }

    let mut maps = vec![0; total_capacity];
    let mut current_capacity = 0;
    let mut attacks_index_shifts = vec![0; 64];
    for i in 0..64 {
        attacks_index_shifts[i] = current_capacity;
        let combs = get_combs(bbs[i]);
        let attacks = get_attacks(i, is_rook, &combs);
        for (j, comb) in combs.iter().enumerate() {
            let magic_index = (comb.wrapping_mul(magics[i]) >> (64 - magic_bits[i])) as usize;
            maps[magic_index + current_capacity] = attacks[j];
        }
        current_capacity += 1 << magic_bits[i];
    }

    (maps, magics, magic_bits.to_vec(), attacks_index_shifts)
}

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
/// Blocker boards are similar to attacks on an empty blocker board combination,
/// except a stop before the last rank/file
pub fn get_blocker_boards(is_rook: bool) -> Vec<u64> {
    let mut bbs = vec![0; 64];
    if is_rook {
        for (i, bb) in bbs.iter_mut().enumerate() {
            for j in (i..56).step_by(8) {
                // up
                set_bit(bb, j);
            }
            for j in (8..i + 1).rev().step_by(8) {
                // down (including 8, i)
                set_bit(bb, j);
            }
            for j in (i & 56) + 1..i + 1 {
                // left (still left-to-right though)
                set_bit(bb, j);
            }
            for j in i..(i | 7) {
                // right
                set_bit(bb, j);
            }
            del_bit(bb, i);
        }
    } else {
        for (i, bb) in bbs.iter_mut().enumerate() {
            for j in (i..).step_by(7).take_while(|j| j / 8 < 7 && j & 7 > 0) {
                // up-left
                set_bit(bb, j);
            }
            for j in (i..).step_by(9).take_while(|j| j / 8 < 7 && j & 7 < 7) {
                // up-right
                set_bit(bb, j);
            }
            let mut j = i;
            while j / 8 > 0 && j & 7 < 7 {
                // down-right
                set_bit(bb, j);
                j -= 7;
            }
            j = i;
            while j / 8 > 0 && j & 7 > 0 {
                // down-left
                set_bit(bb, j);
                j -= 9;
            }
            del_bit(bb, i);
        }
    }
    bbs
}

/// Generate all possible permutations of blockers by given blocker board
fn get_combs(bb: u64) -> Vec<u64> {
    let bits = bb.count_ones();
    let mut combs = vec![0; 1 << bits];
    for (i, comb) in combs.iter_mut().enumerate() {
        let mut mask = bb;
        let mut bit: usize = 0;
        while mask != 0 {
            let csq = pop_bit(&mut mask);
            if i & (1 << bit) != 0 {
                set_bit(comb, csq);
            }
            bit += 1;
        }
    }
    combs
}

/* Example of an attack result by given comb for a rook:
    (we can always do 'ATTACKS &= !ALLY_PIECES' stuff later in the board/engine logic)

    0 0 0 1 0 0 0 0    0 0 0 1 0 0 0 0          *the '1' at the upper rank is not possible
    0 0 0 0 0 0 0 0    0 0 0 1 0 0 0 0           because of the blocker board generation,
    0 0 0 0 0 0 0 0    0 0 0 1 0 0 0 0           but this example shows why
    0 1 0 T 0 0 0 0    0 1 1 T 1 1 1 1           it's designed such way
    0 0 0 1 0 0 0 0    0 0 0 1 0 0 0 0
    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0
    0 0 0 1 0 0 0 0    0 0 0 0 0 0 0 0 
    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0
*/
fn get_attacks(sq: usize, is_rook: bool, combs: &[u64]) -> Vec<u64> {
    let cnt = combs.len();
    let mut attacks = vec![0; cnt];
    for i in 0..cnt {
        // attacks[i] = 0;
        if is_rook {
            for j in (sq..64).step_by(8) {
                // up
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) != 0 {
                    break;
                }
            }
            for j in (0..sq + 1).rev().step_by(8) {
                // down
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) != 0 {
                    break;
                }
            }
            for j in (sq & 56..sq + 1).rev() {
                // left
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) != 0 {
                    break;
                }
            }
            for j in sq..(sq | 7) + 1 {
                // right
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) != 0 {
                    break;
                }
            }
            del_bit(&mut attacks[i], sq);
        } else {
            let mut j = sq;
            loop {
                // up-right
                j += 7;
                if j & 7 >= sq & 7 || j > 62 {
                    break;
                }
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) != 0 {
                    break;
                }
            }
            j = sq;
            loop {
                // up-left
                j += 9;
                if j & 7 <= sq & 7 || j > 63 {
                    break;
                }
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) != 0 {
                    break;
                }
            }
            j = sq;
            loop {
                // down-left
                if j < 8 {
                    break;
                }
                j -= 7;
                if j & 7 <= sq & 7 {
                    break;
                }
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) != 0 {
                    break;
                }
            }
            j = sq;
            loop {
                // down-right
                if j < 9 {
                    break;
                }
                j -= 9;
                if j & 7 >= sq & 7 {
                    break;
                }
                set_bit(&mut attacks[i], j);
                if get_bit(combs[i], j) != 0 {
                    break;
                }
            }
        }
    }
    attacks
}

/// Generate a random number with low amount of 1's
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
    use crate::frame::util::{bb_to_str, str_to_bb};


    #[test]
    fn test_magic_blockers() {
        let bbsr = get_blocker_boards(true);
        assert_eq!(bbsr.len(), 64);
        let bbsb = get_blocker_boards(false);
        assert_eq!(bbsb.len(), 64);

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
        let rook_combs = get_combs(rook_bb);
        assert_eq!(rook_combs.len(), 1 << 12);
        
        // bbsb[59]
        let bishop_bb = str_to_bb("0000000000010100001000100100000000000000000000000000000000000000");
        let bishop_combs = get_combs(bishop_bb);
        assert_eq!(bishop_combs.len(), 1 << bishop_bb.count_ones());

        assert_eq!(0, rook_combs[0]);
        assert_eq!(rook_bb, *rook_combs.last().unwrap());
        assert_eq!(0, bishop_combs[0]);
        assert_eq!(bishop_bb, *bishop_combs.last().unwrap());
        
        pop_bit(&mut rook_bb);
        assert_eq!(rook_bb, rook_combs[(1 << rook_bits) - 2]);
    }

    #[test]
    fn test_magic_attacks() {
        let combr = vec![str_to_bb("0000000000000000000001000011000000000100000000000000010000000000")];
        let attackr = get_attacks(34, true, &combr);
        assert_eq!(combr.len(), attackr.len());
        assert_eq!("0000000000000000000001000001101100000100000000000000000000000000", bb_to_str(attackr[0]));
    
        let combb = vec![str_to_bb("0000000000100000000001000000000000000000001000000100000000000000")];
        let attackb = get_attacks(35, false, &combb);
        assert_eq!(combb.len(), attackb.len());
        assert_eq!("0000000000100000000101000000000000010100001000100000000100000000", bb_to_str(attackb[0]));

        let combb = vec![str_to_bb("0000000000001000000000000000000000000000000000000000010000000000")];
        let attackb = get_attacks(37, false, &combb);
        assert_eq!("0000000010001000010100000000000001010000100010000000010000000000", bb_to_str(attackb[0]));

        let attackb = get_attacks(63, false, &combb);
        assert_eq!("0000000001000000001000000001000000001000000001000000001000000001", bb_to_str(attackb[0]));

        let attackb = get_attacks(56, false, &combb);
        assert_eq!("0000000000000010000001000000100000010000001000000100000010000000", bb_to_str(attackb[0]));

        let attackb = get_attacks(8, false, &combb);
        assert_eq!("0100000000100000000100000000100000000100000000100000000000000010", bb_to_str(attackb[0]));

        let attackb = get_attacks(7, false, &combb);
        assert_eq!("0000000100000010000001000000100000010000001000000100000000000000", bb_to_str(attackb[0]));

        let attackb = get_attacks(57, false, &combb);
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