use std::path::Path;
use crate::util::*;

pub fn init_leaping_attacks() {
    if Path::new(PATH_AMK).exists() {
        println!("Found king attack maps.");
    } else {
        println!("No king attack maps! Creating file at: {}", PATH_AMK);
        let mut attacks = vec![0; 64];
        init_attacks_king(&mut attacks);
        vector_to_file(&attacks, PATH_AMK);
    }

    if Path::new(PATH_AMN).exists() {
        println!("Found knight attack maps.");
    } else {
        println!("No knight attack maps! Creating file at: {}", PATH_AMN);
        let mut attacks = vec![0; 64];
        init_attacks_knight(&mut attacks);
        vector_to_file(&attacks, PATH_AMN);
    }

    if Path::new(PATH_AMP).exists() && Path::new(PATH_AMP2).exists() {
        println!("Found pawn attack maps.");
    } else {
        println!("No pawn attack maps! Creating files at: {}, {}", PATH_AMP, PATH_AMP2);
        let mut white = vec![0; 64];
        let mut black = vec![0; 64];
        init_attacks_pawns(&mut white, &mut black);
        vector_to_file(&white, PATH_AMP);
        vector_to_file(&black, PATH_AMP2);
    }
}

fn init_attacks_king(attacks: &mut[u64]) {
    for (i, attack) in attacks.iter_mut().enumerate() {
        let down  = i > 7;
        let up    = i < 56;
        let left  = i % 8 < 7;
        let right = i % 8 > 0;

        if right         { set_bit(attack, i - 1 )};
        if right && down { set_bit(attack, i - 9 )};
        if down          { set_bit(attack, i - 8 )};
        if down && left  { set_bit(attack, i - 7 )};
        if left          { set_bit(attack, i + 1 )};
        if left && up    { set_bit(attack, i + 9 )};
        if up            { set_bit(attack, i + 8 )};
        if up && right   { set_bit(attack, i + 7 )};
    }
}

fn init_attacks_knight(attacks: &mut[u64]) {
    for (i, attack) in attacks.iter_mut().enumerate() {
        if i > 16 && i % 8 > 0 { set_bit(attack, i - 17 )};
        if i > 15 && i % 8 < 7 { set_bit(attack, i - 15 )};
        if i >  9 && i % 8 > 1 { set_bit(attack, i - 10 )};
        if i >  7 && i % 8 < 6 { set_bit(attack, i -  6 )};
        if i < 56 && i % 8 > 1 { set_bit(attack, i +  6 )};
        if i < 54 && i % 8 < 6 { set_bit(attack, i + 10 )};
        if i < 48 && i % 8 > 0 { set_bit(attack, i + 15 )};
        if i < 47 && i % 8 < 7 { set_bit(attack, i + 17 )};
    }
}

fn init_attacks_pawns(white: &mut[u64], black: &mut[u64]) {
    for (i, attack) in white.iter_mut().enumerate() {
        if i < 56 {
            if i % 8 > 0 {
                set_bit(attack, i + 7);
            }
            if i % 8 < 7 {
                set_bit(attack, i + 9);
            }
        }
    }
    for (i, attack) in black.iter_mut().enumerate() {
        if i > 7 {
            if i % 8 > 0 {
                set_bit(attack, i - 9);
            }
            if i % 8 < 7 {
                set_bit(attack, i - 7);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::bb_to_str;

    #[test]
    fn test_leaping_attacks_king() {
        let mut attacks = vec![0; 64];
        init_attacks_king(&mut attacks);

        assert_eq!("0000001000000011000000000000000000000000000000000000000000000000", bb_to_str(attacks[56]));
        assert_eq!("0100000011000000000000000000000000000000000000000000000000000000", bb_to_str(attacks[63]));
        assert_eq!("0000000000000000000000000000000000000000000000001100000001000000", bb_to_str(attacks[ 7]));
        assert_eq!("0000000000000000000000000000000000000000000000000000001100000010", bb_to_str(attacks[ 0]));
        assert_eq!("0000000000000000000000000000000001110000010100000111000000000000", bb_to_str(attacks[21]));
    }

    #[test]
    fn test_leaping_attacks_knight() {
        let mut attacks = vec![0; 64];
        init_attacks_knight(&mut attacks);

        assert_eq!("0000000000000000000000000000000010100000000100000000000000010000", bb_to_str(attacks[14]));
        assert_eq!("0000101000010001000000000001000100001010000000000000000000000000", bb_to_str(attacks[42]));
        assert_eq!("0000100000000000000010000000010100000000000000000000000000000000", bb_to_str(attacks[49]));
        assert_eq!("0101000010001000000000001000100001010000000000000000000000000000", bb_to_str(attacks[45]));
        assert_eq!("0000000000101000010001000000000001000100001010000000000000000000", bb_to_str(attacks[36]));
    }

    #[test]
    fn test_leaping_attacks_pawn() {
        let mut white = vec![0; 64];
        let mut black = vec![0; 64];
        init_attacks_pawns(&mut white, &mut black);

        assert_eq!("0000000000000000000000000000000000000000010000000000000000000000", bb_to_str(white[15]));
        assert_eq!("0000001000000000000000000000000000000000000000000000000000000000", bb_to_str(white[48]));
        assert_eq!("0000000000101000000000000000000000000000000000000000000000000000", bb_to_str(white[44]));
        assert_eq!("0000000000000000010000000000000000000000000000000000000000000000", bb_to_str(black[55]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000000000010", bb_to_str(black[ 8]));
        assert_eq!("0000000000000000000000000010100000000000000000000000000000000000", bb_to_str(black[44]));
    }
}