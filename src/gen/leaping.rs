use crate::frame::util::set_bit;


pub fn get_attacks_king() -> Vec<u64> {
    let mut attacks = vec![0; 64];
    for (i, attack) in attacks.iter_mut().enumerate() {
        let down  = i > 7;
        let up    = i < 56;
        let left  = i & 7 < 7;
        let right = i & 7 != 0;

        if right         { set_bit(attack, i - 1 ) };
        if right && down { set_bit(attack, i - 9 ) };
        if down          { set_bit(attack, i - 8 ) };
        if down && left  { set_bit(attack, i - 7 ) };
        if left          { set_bit(attack, i + 1 ) };
        if left && up    { set_bit(attack, i + 9 ) };
        if up            { set_bit(attack, i + 8 ) };
        if up && right   { set_bit(attack, i + 7 ) };
    }
    attacks
}

pub fn get_attacks_knight() -> Vec<u64> {
    let mut attacks = vec![0; 64];
    for (i, attack) in attacks.iter_mut().enumerate() {
        if i > 16 && i & 7 > 0 { set_bit(attack, i - 17 ) };
        if i > 15 && i & 7 < 7 { set_bit(attack, i - 15 ) };
        if i >  9 && i & 7 > 1 { set_bit(attack, i - 10 ) };
        if i >  7 && i & 7 < 6 { set_bit(attack, i -  6 ) };
        if i < 56 && i & 7 > 1 { set_bit(attack, i +  6 ) };
        if i < 54 && i & 7 < 6 { set_bit(attack, i + 10 ) };
        if i < 48 && i & 7 > 0 { set_bit(attack, i + 15 ) };
        if i < 47 && i & 7 < 7 { set_bit(attack, i + 17 ) };
    }
    attacks
}

pub fn get_attacks_pawns_white() -> Vec<u64> {
    let mut white = vec![0; 64];
    for (i, attack) in white.iter_mut().enumerate() {
        if i < 56 {
            if i & 7 > 0 {
                set_bit(attack, i + 7);
            }
            if i & 7 < 7 {
                set_bit(attack, i + 9);
            }
        }
    }
    white
}

pub fn get_attacks_pawns_black() -> Vec<u64> {
    let mut black = vec![0; 64];
    for (i, attack) in black.iter_mut().enumerate() {
        if i > 7 {
            if i & 7 > 0 {
                set_bit(attack, i - 9);
            }
            if i & 7 < 7 {
                set_bit(attack, i - 7);
            }
        }
    }
    black
}

pub fn get_steps_pawns_white() -> Vec<u64> {
    let mut white = vec![0; 64];
    for (i, step) in white.iter_mut().enumerate() {
        if i < 56 {
            set_bit(step, i + 8);
        }
    }
    white
}

pub fn get_steps_pawns_black() -> Vec<u64> {
    let mut black = vec![0; 64];
    for (i, step) in black.iter_mut().enumerate() {
        if i > 7 {
            set_bit(step, i - 8);
        }
    }
    black
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::util::bb_to_str;

    #[test]
    fn test_leaping_attacks_king() {
        let attacks = get_attacks_king();
        assert_eq!(attacks.len(), 64);

        assert_eq!("0000001000000011000000000000000000000000000000000000000000000000", bb_to_str(attacks[56]));
        assert_eq!("0100000011000000000000000000000000000000000000000000000000000000", bb_to_str(attacks[63]));
        assert_eq!("0000000000000000000000000000000000000000000000001100000001000000", bb_to_str(attacks[ 7]));
        assert_eq!("0000000000000000000000000000000000000000000000000000001100000010", bb_to_str(attacks[ 0]));
        assert_eq!("0000000000000000000000000000000001110000010100000111000000000000", bb_to_str(attacks[21]));
    }

    #[test]
    fn test_leaping_attacks_knight() {
        let attacks = get_attacks_knight();
        assert_eq!(attacks.len(), 64);

        assert_eq!("0000000000000000000000000000000010100000000100000000000000010000", bb_to_str(attacks[14]));
        assert_eq!("0000101000010001000000000001000100001010000000000000000000000000", bb_to_str(attacks[42]));
        assert_eq!("0000100000000000000010000000010100000000000000000000000000000000", bb_to_str(attacks[49]));
        assert_eq!("0101000010001000000000001000100001010000000000000000000000000000", bb_to_str(attacks[45]));
        assert_eq!("0000000000101000010001000000000001000100001010000000000000000000", bb_to_str(attacks[36]));
    }

    #[test]
    fn test_leaping_attacks_pawn() {
        let white = get_attacks_pawns_white();
        assert_eq!(white.len(), 64);
        let black = get_attacks_pawns_black();
        assert_eq!(black.len(), 64);

        assert_eq!("0000000000000000000000000000000000000000010000000000000000000000", bb_to_str(white[15]));
        assert_eq!("0000001000000000000000000000000000000000000000000000000000000000", bb_to_str(white[48]));
        assert_eq!("0000000000101000000000000000000000000000000000000000000000000000", bb_to_str(white[44]));
        assert_eq!("0000000000000000010000000000000000000000000000000000000000000000", bb_to_str(black[55]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000000000010", bb_to_str(black[ 8]));
        assert_eq!("0000000000000000000000000010100000000000000000000000000000000000", bb_to_str(black[44]));
    
        let sw = get_steps_pawns_white();
        assert_eq!(sw.len(), 64);
        let sb = get_steps_pawns_black();
        assert_eq!(sb.len(), 64);

        assert_eq!("1000000000000000000000000000000000000000000000000000000000000000", bb_to_str(sw[55]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000100000000", bb_to_str(sw[ 0]));
        assert_eq!("0000000000000000000000000000000000010000000000000000000000000000", bb_to_str(sw[20]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000100000000", bb_to_str(sb[16]));
        assert_eq!("0000000000000000000000000000000000010000000000000000000000000000", bb_to_str(sb[36]));
        assert_eq!("0000000000000000000000000000000000000000000000000000000010000000", bb_to_str(sb[15]));
    }
}