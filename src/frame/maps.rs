use crate::gen::{leaping::*, magic::*, secondary::*};


pub struct Maps {
    pub attacks_rook:        Vec<u64>,
    pub ais_rook:           [usize; 64],
    pub bbs_rook:           [u64; 64],
    pub magics_rook:        [u64; 64],
    pub magic_bits_rook:    [usize; 64],

    pub attacks_bishop:      Vec<u64>,
    pub ais_bishop:         [usize; 64],
    pub bbs_bishop:         [u64; 64],
    pub magics_bishop:      [u64; 64],
    pub magic_bits_bishop:  [usize; 64],

    pub attacks_king:       [u64; 64],
    pub attacks_knight:     [u64; 64],
    pub attacks_pawns:     [[u64; 64]; 2],  // white / black
    pub steps_pawns:       [[u64; 64]; 2],  // white / black
    
    pub files:              [u64; 64],
    pub ranks:              [u64; 64],
    pub flanks:             [u64; 64],
    pub fwd:               [[u64; 64]; 2],
    pub rad2:               [u64; 64]
}

impl Default for Maps {
    fn default() -> Maps {
        let bbs_rook = get_blocker_boards(true);
        let pre_calculated_rook = vec![9259401108760043540, 9241386710779039746, 1261043081789050882, 5008020380640642176, 9943957941625421856, 2341899448650566144, 1224983498855547392, 5512422163845505280, 4685010259524485412, 99923758738833536, 144396770430951425, 4765089949470957576, 281543830406400, 92886750904976432, 2392541597139072, 432908515591389314, 225215715500621952, 1170938652432302144, 13916123123721699396, 4755942494015850497, 650770696045331072, 564053760278536, 7638320472570003976, 36030996046446852, 12213797925704187904, 9808875182448726019, 2305985954316296261, 144714426208751616, 721701859613999744, 101489905422835776, 288373329844240648, 325103881568682244, 4620834230056779809, 1170936180234002432, 198457452990046210, 193516269479936, 2251817001945216, 4623085772189206016, 9817849455477522696, 1169338400916, 108086666471718912, 2458982989806190603, 2305861159747715136, 38280734540103936, 1225260590935638034, 4398080098432, 4611757497152438280, 9237521137737793537, 2886948167893385344, 4611756389321146496, 581106257652351360, 650779010969698560, 2763090034753792, 578716950197207168, 2306124544321258752, 720857417507799296, 2382544941442146569, 18700289853570, 292883786688004130, 35218815717377, 577023839763435522, 77125002880680962, 1208530406874808580, 76561332194804738];
        let (attack_maps_rook, magics_rook, magic_bits_rook, ais_rook) = get_magic_maps(true, Some(&pre_calculated_rook), Some(&bbs_rook), None);

        let bbs_bishop = get_blocker_boards(false);
        let pre_calculated_bishop = vec![1747440672543213282, 4684892224205424768, 2260665708380416, 9224569405554573316, 1130435392569361, 2342155103349243968, 74835578326016, 2882585512512864260, 37159389244953604, 6088869788648514176, 18032076603271168, 2542113868858368, 1442279988555481608, 576462058109796352, 2596396121062522884, 9269536136362070148, 2314852976710131968, 7504123016881378384, 580559386706440, 13850820671732514820, 1730789666690957316, 298152996163815680, 5192792280442937409, 9223653520493118016, 4901114037888223232, 2306128882523177472, 20268397414842384, 571746113568776, 9808842189629837313, 5764679004208529952, 38844251165167616, 36099717804988416, 10378580561081157928, 2310918500922692112, 1163054760535261696, 2316681996988317824, 4505807241617416, 1173188837875648513, 3603310719044454913, 2306137822211410240, 1277701532991488, 578862120198873608, 2850485237260356, 9297857498664945664, 10417257015655010561, 4756366510249740298, 5427352670568576, 757224879278784544, 45656688304848896, 218064348520584, 9895255014430212353, 1443986565684281344, 4504836611801090, 9223407229854680064, 292770019186933760, 4647088109683264, 1153485558501147648, 351985589162000, 1188950370899462144, 200432382267101440, 2306406337193476608, 422526267181186, 5909584865800552516, 38368699501052424];
        let (attack_maps_bishop, magics_bishop, magic_bits_bishop, ais_bishop) = get_magic_maps(false, Some(&pre_calculated_bishop), Some(&bbs_bishop), None);

        let attack_maps_king = get_attacks_king();
        let attack_maps_knight = get_attacks_knight();
        let attack_maps_pawns = [get_attacks_pawns_white(), get_attacks_pawns_black()];
        let steps_pawns = [get_steps_pawns_white(), get_steps_pawns_black()];

        let files = get_files();
        let ranks = get_ranks();
        let flanks = get_flanks();
        let fwd = [get_forward_field_white(), get_forward_field_black()];
        let rad2 = get_radius_2();

        Self {
            attacks_rook: attack_maps_rook,
            ais_rook: ais_rook.try_into().unwrap(),
            bbs_rook: bbs_rook.try_into().unwrap(),
            magics_rook: magics_rook.try_into().unwrap(),
            magic_bits_rook: magic_bits_rook.try_into().unwrap(),
            attacks_bishop: attack_maps_bishop, 
            ais_bishop: ais_bishop.try_into().unwrap(),
            bbs_bishop: bbs_bishop.try_into().unwrap(),
            magics_bishop: magics_bishop.try_into().unwrap(),
            magic_bits_bishop: magic_bits_bishop.try_into().unwrap(),
            attacks_king: attack_maps_king.try_into().unwrap(),
            attacks_knight: attack_maps_knight.try_into().unwrap(),
            attacks_pawns: [attack_maps_pawns[0].clone().try_into().unwrap(), attack_maps_pawns[1].clone().try_into().unwrap()],
            steps_pawns: [steps_pawns[0].clone().try_into().unwrap(), steps_pawns[1].clone().try_into().unwrap()],
            files: files.try_into().unwrap(),
            ranks: ranks.try_into().unwrap(),
            flanks: flanks.try_into().unwrap(),
            fwd: [fwd[0].clone().try_into().unwrap(), fwd[1].clone().try_into().unwrap()],
            rad2: rad2.try_into().unwrap()
        }
    }
}