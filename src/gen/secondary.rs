// A module to generate additional bit masks that are useful to eval()

// Init them LAST!
// Will crash if no leaping attack maps (PATH_AMN) are present!

use crate::util::*;

pub fn init_secondary_maps() {
    init64(init_ranks, PATH_RNK);
    init64(init_files, PATH_FLS);
    init64(init_passing_piece_maps_white, PATH_PPM);
    init64(init_passing_piece_maps_black, PATH_PPM2);
    init64(init_passing_piece_blocked_maps_white, PATH_PBM);
    init64(init_passing_piece_blocked_maps_black, PATH_PBM2);
    init64(init_double_attack_maps_knight, PATH_DAMN);
}

fn init_double_attack_maps_knight(attacks: &mut[u64]) {
    let kas = file_to_vector(PATH_AMN);     // !!!
    for i in 0..64 {
        let mut mask = kas[i];
        while mask != 0 {
            let csq = pop_bit(&mut mask);
            attacks[i] |= kas[csq];
        }
        del_bit(&mut attacks[i], i);
    }
}

fn init_ranks(ranks: &mut[u64]) {
    for i in 0..64 {
        let offset = i & 56;
        for j in 0..8 {
            set_bit(&mut ranks[i], j + offset);
        }
        del_bit(&mut ranks[i], i);
    }
}

fn init_files(files: &mut[u64]) {
    for i in 0..64 {
        let mut j = i & 7;
        while j < 64 {
            set_bit(&mut files[i], j);
            j += 8;
        }
        del_bit(&mut files[i], i);
    }
}

fn init_passing_piece_maps_white(map: &mut[u64]) {
    for i in 0..64 {
        let file = i & 7;
        let mut j = i + 8;
        while j < 64 {
            set_bit(&mut map[i], j);
            if file > 0 {
                set_bit(&mut map[i - 1], j);
            }
            if file < 7 {
                set_bit(&mut map[i + 1], j);
            }
            j += 8;
        }
    }
}

fn init_passing_piece_maps_black(map: &mut[u64]) {
    for i in 0..64 {
        let file = i & 7;
        let mut j = file;
        while j < i {
            set_bit(&mut map[i], j);
            if file > 0 {
                set_bit(&mut map[i - 1], j);
            }
            if file < 7 {
                set_bit(&mut map[i + 1], j);
            }
            j += 8;
        }
    }
}

fn init_passing_piece_blocked_maps_white(map: &mut[u64]) {
    for i in 0..64 {
        let file = i & 7;
        let mut j = i + 8;
        while j < 64 {
            if file > 0 {
                set_bit(&mut map[i - 1], j);
            }
            if file < 7 {
                set_bit(&mut map[i + 1], j);
            }
            j += 8;
        }
    }
}

fn init_passing_piece_blocked_maps_black(map: &mut[u64]) {
    for i in 0..64 {
        let file = i & 7;
        let mut j = file;
        while j < i {
            if file > 0 {
                set_bit(&mut map[i - 1], j);
            }
            if file < 7 {
                set_bit(&mut map[i + 1], j);
            }
            j += 8;
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::bb_to_str;

    #[test]
    fn test_secondary_double_attacks_knight() {
        let mut map = vec![0; 64];
        init_double_attack_maps_knight(&mut map);

        assert_eq!("0101010010101010000100011010101001000101101010100001000110101010", bb_to_str(map[28]));
        assert_eq!("0001010000001000000100010000101000000101000000000000000000000000", bb_to_str(map[56]));
    }

    #[test]
    fn test_secondary_passing_pieces() {
        let mut ppw = vec![0; 64];
        let mut ppb = vec![0; 64];
        let mut pbw = vec![0; 64];
        let mut pbb = vec![0; 64];
        init_passing_piece_maps_white(&mut ppw);
        init_passing_piece_maps_black(&mut ppb);
        init_passing_piece_blocked_maps_white(&mut pbw);
        init_passing_piece_blocked_maps_black(&mut pbb);

        assert_eq!("1110000011100000111000000000000000000000000000000000000000000000", bb_to_str(ppw[38]));
        assert_eq!("1010000010100000101000000000000000000000000000000000000000000000", bb_to_str(pbw[38]));
        assert_eq!("0000000000000000000000000000000011100000111000001110000011100000", bb_to_str(ppb[38]));
        assert_eq!("0000000000000000000000000000000010100000101000001010000010100000", bb_to_str(pbb[38]));

        assert_eq!("1100000011000000110000001100000011000000110000000000000000000000", bb_to_str(ppw[15]));
        assert_eq!("0100000001000000010000000100000001000000010000000000000000000000", bb_to_str(pbw[15]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000011000000", bb_to_str(ppb[15]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000001000000", bb_to_str(pbb[15]));

        assert_eq!("0000001100000011000000000000000000000000000000000000000000000000", bb_to_str(ppw[40]));
        assert_eq!("0000001000000010000000000000000000000000000000000000000000000000", bb_to_str(pbw[40]));
        assert_eq!("0000000000000000000000000000001100000011000000110000001100000011", bb_to_str(ppb[40]));
        assert_eq!("0000000000000000000000000000001000000010000000100000001000000010", bb_to_str(pbb[40]));
    }

    #[test]
    fn test_secondary_ranks_files() {
        let mut ranks = vec![0; 64];
        let mut files = vec![0; 64];
        init_ranks(&mut ranks);
        init_files(&mut files);
        
        assert_eq!("0000000100000001000000010000000100000001000000010000000100000000", bb_to_str(files[ 0]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000011111110", bb_to_str(ranks[ 0]));
        assert_eq!("0000100000001000000010000000000000001000000010000000100000001000", bb_to_str(files[35]));
        assert_eq!("0000000000000000000000001111011100000000000000000000000000000000", bb_to_str(ranks[35]));

        for i in 0..64 {
            assert_eq!(ranks[i] & files[i], 0);
            assert_eq!(get_bit(ranks[i], i), 0);
            assert_eq!(get_bit(files[i], i), 0);
        }
    }
}