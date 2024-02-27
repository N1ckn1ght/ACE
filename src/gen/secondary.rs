// A module to generate additional bit masks that are useful to eval()
// Init them LAST!

use crate::frame::util::*;

pub fn init_secondary_maps() {
    init64(init_ranks, PATH_RNK);
    init64(init_files, PATH_FLS);
    init64(init_flanks, PATH_FKS);
    init64(init_forward_field_white, PATH_FFD);
    init64(init_forward_field_black, PATH_FFD2);
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
    
}

fn init_forward_field_white(map: &mut[u64]) {

}

fn init_forward_field_black(map: &mut[u64]) {

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::util::bb_to_str;
    
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