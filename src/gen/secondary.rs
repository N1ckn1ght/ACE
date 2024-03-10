// A module to generate additional bit masks that are useful to eval()

use crate::frame::util::*;

pub fn init_secondary_maps() {
    init64(init_ranks, PATH_RNK);
    init64(init_files, PATH_FLS);
    init64(init_flanks, PATH_FKS);
    init64(init_forward_field_white, PATH_FWD);
    init64(init_forward_field_black, PATH_FWD2);
    init64(init_radius_2, PATH_RAD2);
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

fn init_flanks(map: &mut[u64]) {
    for i in 0..64 {
        let file = i & 7;
        for j in (file..64).step_by(8) {
            if file != 0 {
                set_bit(&mut map[i], j - 1);
            }
            if file != 7 {
                set_bit(&mut map[i], j + 1);
            }
        }
    }
}

fn init_forward_field_white(map: &mut[u64]) {
    for i in 0..56 {
        for j in ((i & 56) + 8)..64 {
            set_bit(&mut map[i], j);
        }
    }
}

fn init_forward_field_black(map: &mut[u64]) {
    for i in 8..64 {
        for j in 0..(i & 56) {
            set_bit(&mut map[i], j);
        }
    }
}

fn init_radius_2(map: &mut[u64]) {
    let mut rad1 = [0; 64];
    for (i, attack) in rad1.iter_mut().enumerate() {
        let down  = i > 7;
        let up    = i < 56;
        let left  = i & 7 < 7;
        let right = i & 7 != 0;

        if right         { set_bit(attack, i - 1 )};
        if right && down { set_bit(attack, i - 9 )};
        if down          { set_bit(attack, i - 8 )};
        if down && left  { set_bit(attack, i - 7 )};
        if left          { set_bit(attack, i + 1 )};
        if left && up    { set_bit(attack, i + 9 )};
        if up            { set_bit(attack, i + 8 )};
        if up && right   { set_bit(attack, i + 7 )};
    }

    for i in 0..64 {
        let mut bits = rad1[i];
        while bits != 0 {
            let sq = pop_bit(&mut bits);
            map[i] |= rad1[sq];
        }
        del_bit(&mut map[i], i);
        // map[i] &= !rad1[i];
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::util::bb_to_str;
    
    #[test]
    fn test_secondary_maps() {
        let mut ranks = vec![0; 64];
        init_ranks(&mut ranks);
        let mut files = vec![0; 64];
        init_files(&mut files);
        
        assert_eq!("0000000100000001000000010000000100000001000000010000000100000000", bb_to_str(files[ 0]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000011111110", bb_to_str(ranks[ 0]));
        assert_eq!("0000100000001000000010000000000000001000000010000000100000001000", bb_to_str(files[35]));
        assert_eq!("0000000000000000000000001111011100000000000000000000000000000000", bb_to_str(ranks[35]));

        let mut flanks = vec![0; 64];
        init_flanks(&mut flanks);

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

        let mut ffdw = vec![0; 64];
        init_forward_field_white(&mut ffdw);
        let mut ffdb = vec![0; 64];
        init_forward_field_black(&mut ffdb);

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
    
        let mut rad2 = vec![0; 64];
        init_radius_2(&mut rad2);

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