/// Search for magic numbers for perfect hashing of board blockers to calculate sliding pieces attacks
/// 
/// The algorithm of feeding in randoms provided by Tord Romstad
/// www.talkchess.com/forum3/viewtopic.php?topic_view=threads&p=175834

use crate::frame::util::*;


fn search_for_magic(
    sq: usize,
    is_rook: bool,
    target: usize,
    bb: u64,
    seed: &mut u64
) -> u64 {
    let bits = bb.count_ones() as usize;

    // generate all possible combinations of blockers (we still don't care about last ranks and files)
    let combs = get_combs(bb, bits);

    // generate attack maps for every combination of blockers (here WE DO CARE about last ranks and files)
    let attacks = get_attacks(sq, is_rook, &combs);

    // bruteforce approach, let's go over random numbers and find magic ones
    let mut magic;
    loop {
        magic = next_random_magic(&mut *seed);

        // fast heuristic optimization
        if u64::count_ones(bb.wrapping_mul(magic) & 0xFF00000000000000) < 6 {
            continue;
        }

        // we try to hash positions and see if there's a collision* or not
        // hashed value should contain necessary information about blockers
        // we are not interested in other pieces placements
        let mut fail = false;
        let mut used = vec![0; attacks.len()];
        for i in 0..combs.len() {
            let index = (combs[i].wrapping_mul(magic) >> (64 - target)) as usize;
            // *except different blocker boards often produce the same attack map
            // and the same hash on the same attack is even better!
            if used[index] > 0 && used[index] != attacks[i] {
                fail = true;
                break;
            }
            used[index] = attacks[i];
        }
        if fail {
            continue;
        }
    }

    magic
}


/* Blocker boards are similar to attacks on an empty blocker board combination,
    except we stop before the last rank/file

   Examples:

    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 1 0 0 0 1 0 0    0 0 0 0 0 0 1 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 0 1 0 1 0 0 0    0 0 0 0 0 1 0 0
    0 1 1 T 1 1 1 0    1 0 0 0 0 0 0 0    0 0 0 T 0 0 0 0    0 0 0 0 1 0 0 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 0 1 0 1 0 0 0    0 0 0 1 0 0 0 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 1 0 0 0 1 0 0    0 0 1 0 0 0 0 0
    0 0 0 1 0 0 0 0    1 0 0 0 0 0 0 0    0 0 0 0 0 0 1 0    0 T 0 0 0 0 0 0
    0 0 0 0 0 0 0 0    T 1 1 1 1 1 1 0    0 0 0 0 0 0 0 0    0 0 0 0 0 0 0 0
*/
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


// Generate all possible permutations of blockers by given blocker board

fn get_combs(bb: u64, bits: usize) -> Vec<u64> {
    let mut combs = vec![0; 1 << bits];
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

pub fn get_magic_maps(
    is_rook: bool,
    magic_numbers: Option<&Vec<u64>>,
    bbss: &Vec<u64>,
    seed: Option<u64>
) -> Vec<u64> {
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

    // it's possible to go further below this bit count, but search of such magic is consuming
    let magic_bits = &comb_bits;

    // call search for magic or use presented values
    let mut magics = match magic_numbers {
        Some(numbers) => {
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
                numbers[i] = search_for_magic(i, is_rook, magic_bits[i], bbss[i], &mut seed);
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
    for i in 0..64 {
        // let mut combs = vec![0; 1 << comb_bits_rook[i]];
        // init_combs(&mut combs, blocker_boards_rook[i]);
        let mut combs = get_combs();
        let mut attacks = vec![0; 1 << magic_bits_rook[i]];
        init_attacks(i, true, &mut attacks, &mut combs, 1 << magic_bits_rook[i]);
        for (j, comb) in combs.iter().enumerate() {
            let magic_index = (comb.wrapping_mul(magics_rook[i]) >> (64 - magic_bits_rook[i])) as usize;
            maps[magic_index + current_capacity] = attacks[j];
        }
        current_capacity += 1 << magic_bits_rook[i];
    }

    vec![]
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
    use crate::frame::util::{bb_to_str, str_to_bb};

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