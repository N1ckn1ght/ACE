use super::util::*;

pub struct Maps {
    pub attacks_rook:        Vec<u64>,
    pub ais_rook:           [usize; 64],
    pub bbs_rook:            Vec<u64>,
    pub magics_rook:        [u64; 64],
    pub magic_bits_rook:    [usize; 64],
    
    pub attacks_bishop:      Vec<u64>,
    pub ais_bishop:         [usize; 64],
    pub bbs_bishop:          Vec<u64>,
    pub magics_bishop:      [u64; 64],
    pub magic_bits_bishop:  [usize; 64],

    pub attacks_king:        Vec<u64>,
    pub attacks_knight:      Vec<u64>,
    pub attacks_pawns:      [Vec<u64>; 2],  // white / black
    
    pub files:               Vec<u64>,
    pub ranks:               Vec<u64>,
    pub piece_passing:      [Vec<u64>; 2],
    pub piece_pb:           [Vec<u64>; 2],
    pub attacks_dn:          Vec<u64>
}

impl Default for Maps {
    fn default() -> Maps {
        let bbs_rook = file_to_vector(PATH_BBR);
        let mut ais_rook            = [0; 64];
        let mut magics_rook         = [0; 64];
        let mut magic_bits_rook     = [0; 64];
        let attack_maps_rook        = file_to_magics(PATH_AMR, &mut magics_rook, &mut magic_bits_rook, &mut ais_rook);
        let bbs_bishop              = file_to_vector(PATH_BBB);
        let mut ais_bishop          = [0; 64];
        let mut magics_bishop       = [0; 64];
        let mut magic_bits_bishop   = [0; 64];
        let attack_maps_bishop      = file_to_magics(PATH_AMB, &mut magics_bishop, &mut magic_bits_bishop, &mut ais_bishop);
        let attack_maps_king        = file_to_vector(PATH_AMK);
        let attack_maps_knight      = file_to_vector(PATH_AMN);
        let attack_maps_pawns       = [file_to_vector(PATH_AMP), file_to_vector(PATH_AMP2)];
        let files                   = file_to_vector(PATH_FLS);
        let ranks                   = file_to_vector(PATH_RNK);
        let piece_passing           = [file_to_vector(PATH_PPM), file_to_vector(PATH_PPM2)];
        let piece_pb                = [file_to_vector(PATH_PBM), file_to_vector(PATH_PBM2)];
        let attacks_dn              = file_to_vector(PATH_DAMN);

        Self {
            attacks_rook: attack_maps_rook,
            ais_rook,
            bbs_rook,
            magics_rook, 
            magic_bits_rook, 
            attacks_bishop: attack_maps_bishop, 
            ais_bishop,
            bbs_bishop, 
            magics_bishop, 
            magic_bits_bishop, 
            attacks_king: attack_maps_king, 
            attacks_knight: attack_maps_knight,
            attacks_pawns: attack_maps_pawns,
            files,
            ranks,
            piece_passing,
            piece_pb,
            attacks_dn
        }
    }
}