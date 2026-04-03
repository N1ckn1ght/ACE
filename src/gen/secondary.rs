use crate::{frame::util::{del_bit, pop_bit, set_bit}, gen::leaping::get_attacks_king};


pub fn get_ranks() -> Vec<u64> {
    let mut ranks = vec![0; 64];
    for i in 0..64 {
        let offset = i & 56;
        for j in 0..8 {
            set_bit(&mut ranks[i], j + offset);
        }
        del_bit(&mut ranks[i], i);
    }
    ranks
}

pub fn get_files() -> Vec<u64> {
    let mut files = vec![0; 64];
    for i in 0..64 {
        let mut j = i & 7;
        while j < 64 {
            set_bit(&mut files[i], j);
            j += 8;
        }
        del_bit(&mut files[i], i);
    }
    files
}

pub fn get_flanks() -> Vec<u64> {
    let mut flanks = vec![0; 64];
    for i in 0..64 {
        let file = i & 7;
        for j in (file..64).step_by(8) {
            if file != 0 {
                set_bit(&mut flanks[i], j - 1);
            }
            if file != 7 {
                set_bit(&mut flanks[i], j + 1);
            }
        }
    }
    flanks
}

pub fn get_forward_field_white() -> Vec<u64> {
    let mut map = vec![0; 64];
    for i in 0..56 {
        for j in ((i & 56) + 8)..64 {
            set_bit(&mut map[i], j);
        }
    }
    map
}

pub fn get_forward_field_black() -> Vec<u64> {
    let mut map = vec![0; 64];
    for i in 8..64 {
        for j in 0..(i & 56) {
            set_bit(&mut map[i], j);
        }
    }
    map
}

pub fn get_radius_2() -> Vec<u64> {
    let mut map = vec![0; 64];

    let rad1 = get_attacks_king();

    for i in 0..64 {
        let mut bits = rad1[i];
        while bits != 0 {
            let sq = pop_bit(&mut bits);
            map[i] |= rad1[sq];
        }
        del_bit(&mut map[i], i);
        // map[i] &= !rad1[i];
    }

    map
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::util::{bb_to_str, get_bit};


    #[test]
    fn test_secondary_maps() {
        let ranks = get_ranks();
        assert_eq!(ranks.len(), 64);
        let files = get_files();
        assert_eq!(files.len(), 64);
        
        assert_eq!("0000000100000001000000010000000100000001000000010000000100000000", bb_to_str(files[ 0]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000011111110", bb_to_str(ranks[ 0]));
        assert_eq!("0000100000001000000010000000000000001000000010000000100000001000", bb_to_str(files[35]));
        assert_eq!("0000000000000000000000001111011100000000000000000000000000000000", bb_to_str(ranks[35]));

        let flanks = get_flanks();
        assert_eq!(flanks.len(), 64);

        assert_eq!("1010000010100000101000001010000010100000101000001010000010100000", bb_to_str(flanks[ 6]));
        assert_eq!("1010000010100000101000001010000010100000101000001010000010100000", bb_to_str(flanks[30]));
        assert_eq!("1010000010100000101000001010000010100000101000001010000010100000", bb_to_str(flanks[62]));
        assert_eq!("0100000001000000010000000100000001000000010000000100000001000000", bb_to_str(flanks[ 7]));
        assert_eq!("0100000001000000010000000100000001000000010000000100000001000000", bb_to_str(flanks[15]));
        assert_eq!("0100000001000000010000000100000001000000010000000100000001000000", bb_to_str(flanks[63]));
        assert_eq!("0000001000000010000000100000001000000010000000100000001000000010", bb_to_str(flanks[ 0]));
        assert_eq!("0000001000000010000000100000001000000010000000100000001000000010", bb_to_str(flanks[56]));
        assert_eq!("0000010100000101000001010000010100000101000001010000010100000101", bb_to_str(flanks[57]));
        assert_eq!("0000010100000101000001010000010100000101000001010000010100000101", bb_to_str(flanks[ 1]));
        assert_eq!("0010100000101000001010000010100000101000001010000010100000101000", bb_to_str(flanks[36]));

        let ffdw = get_forward_field_white();
        assert_eq!(ffdw.len(), 64);
        let ffdb = get_forward_field_black();
        assert_eq!(ffdb.len(), 64);

        assert_eq!("0000000000000000000000000000000000000000000000000000000000000000", bb_to_str(ffdw[56]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000000000000", bb_to_str(ffdw[63]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000000000000", bb_to_str(ffdb[ 0]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000000000000", bb_to_str(ffdb[ 7]));
        assert_eq!("1111111100000000000000000000000000000000000000000000000000000000", bb_to_str(ffdw[55]));
        assert_eq!("1111111100000000000000000000000000000000000000000000000000000000", bb_to_str(ffdw[48]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000011111111", bb_to_str(ffdb[15]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000011111111", bb_to_str(ffdb[ 8]));
        assert_eq!("1111111111111111111111110000000000000000000000000000000000000000", bb_to_str(ffdw[37]));
        assert_eq!("1111111111111111111111110000000000000000000000000000000000000000", bb_to_str(ffdw[34]));
        assert_eq!("0000000000000000000000000000000000000000111111111111111111111111", bb_to_str(ffdb[28]));
        assert_eq!("0000000000000000000000000000000000000000111111111111111111111111", bb_to_str(ffdb[25]));
    
        let rad2 = get_radius_2();
        assert_eq!(rad2.len(), 64);

        assert_eq!("0000000000000000000000001111100011111000110110001111100011111000", bb_to_str(rad2[21]));
        assert_eq!("0000000000000000000001110000011100000110000001110000011100000000", bb_to_str(rad2[24]));
        assert_eq!("0110000011100000111000000000000000000000000000000000000000000000", bb_to_str(rad2[63]));
        assert_eq!("0000000000000000000000000000000000000000001111100011111000110110", bb_to_str(rad2[ 3]));

        for i in 0..64 {
            assert_eq!(ranks[i] & files[i], 0);
            assert_eq!(get_bit(ranks[i], i), 0);
            assert_eq!(get_bit(files[i], i), 0);
            assert_eq!(get_bit(flanks[i], i), 0);
            assert_eq!(get_bit(ffdw[i], i), 0);
            assert_eq!(get_bit(ffdb[i], i), 0);
            assert_eq!(get_bit(rad2[i], i), 0);
        }
    }
}