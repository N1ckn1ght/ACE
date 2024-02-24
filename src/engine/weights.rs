pub struct Weights {
    pub pieces_weights_square_related:  [[[i32; 64]; 6]; 2],
    pub pieces_weights_const:            [[i32;  6]; 2],
	pub mobility_base:					   i32,
    pub turn_factor:					   i32,
	pub turn_add_pre:					   i32,
	pub bad_pawn_penalty_pre:			  [i32;  2],
	pub outpost_pre:					  [i32;  2],
	pub good_pawn_reward_pre:			  [i32;  2],
	pub bishop_pin_pre:					   i32,
	pub bishop_align_at_king_pre:		  [i32;  2],
	pub rook_align_at_king_pre:			  [i32;  2],
	pub rook_connected_pre:				   i32,
	pub queen_any_battery_pre:			   i32,
	pub queen_strike_possible_pre:		   i32,
	pub knight_seems_promising_pre:		   i32
}

impl Default for Weights {
    fn default() -> Weights {

        /* 2-staged static piece positional weights
			- they will be tuned, for now I need to focus no search */
		/* Inspiration for a current implementation was taken from:
			- Simplified Evaluation Function by Tomasz Michniewski
			- PeSTO's Evaluation Function by Tom Kerrigan, Pawel Koziol, Ronald Friederich (The OG!!)
			Note: some modifications are ill-intended for fun sake, copying is not advised :D */

        let pieces_weights_square_related = [
			// opening or middlegame
			[
                // pawns
				[
					   0,    0,    0,    0,    0,    0,    0,    0,
					  96,  121,   70,   95,   71,   12,   60,    0,
					  -5,    5,   25,   30,   30,   50,   25,   10,
					 -15,    5,    5,   20,   25,   10,   10,  -15,
					 -20,   -2,    0,   15,   20,    0,   10,  -20,
					 -25,   -3,  -10,    0,    0,  -30,   30,    0,
					 -30,   -1,  -15,  -15,  -15,   30,   30,  -15,
					   0,    0,    0,    0,    0,    0,    0,    0
				],
                // knights
				[
					-163,  -90,  -30,  -49,    1,  -80,  -20, -103,
					 -72,  -40,   71,   35,   20,   60,    1,   -5,
					 -40,   60,   40,   65,   85,  130,   72,   40,
					  -5,   20,   20,   54,   40,   70,   22,   20,
					 -10,    0,   15,   21,   21,   20,   20,   -2,
					 -30,   -8,   15,   10,   15,   20,   25,  -15,
					 -30,  -50,  -15,    1,    1,    0,  -10,  -20,
					-110,  -50,  -60,   -10, -10,   -2,  -50,  -25
				],
                // bishops
				[
					 -30,  -65,  -50,  -35,  -25,  -30,  -60,  -10,
					 -25,   20,  -15,   -5,   30,   60,   20,  -40,
					 -10,   30,   40,   40,   35,   50,   37,    0,
					   0,   15,   11,   50,   37,   37,   15,    0,
					   0,   10,   30,   20,   20,   12,   10,    0,
					   2,   15,   15,    5,    0,    0,    5,    6,
					   0,   15,    5,    0,    1,   15,   30,    0,
					 -30,    0,  -15,  -15,  -15,  -10,  -40,  -25
				],
                // rooks
				[
					  30,   40,   30,   50,   60,   10,   30,   40,
					  30,   30,   60,   60,   80,   60,   30,   40,
					  -5,   15,   20,   35,   20,   45,   50,   13,
					 -25,  -10,    5,   20,   20,   25,   -5,  -20,
					 -30,  -20,   -5,    0,    5,   -5,    5,  -25,
					 -46,  -20,  -15,  -10,    5,    0,   -5,  -30,
					 -40,  -12,  -13,   -1,    0,   10,   -7,  -50,
					 -15,   -9,    3,   20,   30,    7,  -39,  -10
				],
                // queens
				[
					 -29,    0,   30,   11,   60,   44,   43,   45,
					 -22,  -40,   -1,    1,  -15,   57,   25,   50,
					 -15,  -15,    5,    5,   30,   55,   47,   56,
					 -25,  -25,  -15,  -15,    0,   15,    0,    0,
					  -5,  -25,  -10,  -10,    0,    0,    0,    0,
					 -10,    5,  -10,   -1,   -4,    0,   11,    1,
					 -30,    0,   11,    2,    7,    5,   -1,    0,
					 -10,  -20,   -2,   10,  -15,  -25,  -30,  -50
				],
                // king
				[
					 -40,  -40,  -40,  -40,  -10,  -40,  -40,  -35,
					 -35,  -40,  -40,  -40,  -40,  -40,  -35,  -30,
					 -30,  -35,  -40,  -40,  -40,  -35,  -40,  -20,
					 -25,  -30,  -35,  -40,  -40,  -30,  -20,  -30,
					 -20,  -25,  -30,  -35,  -35,  -20,  -25,  -20,
					 -15,  -20,  -25,  -30,   -5,  -30,  -20,  -10,
					   0,    1,   -5,  -25,  -10,  -10,    3,    2,
					 -20,   30,   20,  -20,   10,  -10,   30,   20
				]
            ],
			// endgame
			[
				// pawns
				[
					   0,    0,    0,    0,    0,    0,    0,    0,
					 195,  180,  160,  140,  150,  130,  165,  190,
					  95,  100,   90,   70,   55,   50,   80,   85,
					  30,   20,   19,    5,    3,    4,   20,   20,
					  15,  -10,   -4,   -5,   -5,  -10,    0,   -1,
					   0,    5,   -5,    0,    0,   -5,    0,  -10,
					 -20,  -20,  -20,  -30,  -30,   -1,  -20,  -30,
					   0,    0,    0,    0,    0,    0,    0,    0
				],
                // knights
				[
					 -60,  -45,  -15,  -30,  -30,  -30,  -45,  -99,
					 -25,  -20,  -10,    0,    0,  -10,  -20,  -45,
					 -20,  -10,   -5,   10,   10,    1,  -10,  -20,
					 -20,    5,   15,   20,   20,   10,    5,  -20,
					 -20,    0,   10,   20,   20,   10,    0,  -20,
					 -20,  -10,    0,   10,   10,    0,  -20,  -20,
					 -45,  -20,  -20,   -5,   -5,  -20,  -20,  -45,
					 -70,  -45,  -30,  -30,  -30,  -30,  -45,  -70
				],
				// bishops
				[
					 -30,   -8,   -5,   -5,   -5,   -5,   -8,  -30,
					  -8,   -1,    0,   -4,   -3,    0,   -1,   -8,
					   5,    0,    5,    0,    0,    6,    0,    5,
					   0,    6,   10,    5,    6,   10,    3,    0,
					   0,    5,   10,   11,    5,   10,    0,    0,
					  -5,    0,    6,    4,    4,    1,    0,   -5,
					 -15,  -10,    0,    0,    0,    0,  -10,  -15,
					 -25,  -15,  -20,  -10,  -10,  -20,  -15,  -25
				],
				// rooks (it kinda makes sense)
				[
					  10,   10,   10,   10,   10,   10,    0,    0,
					  10,   10,   10,   10,    0,    0,    0,    0,
					   0,    0,    0,    0,    0,    0,    0,    0,
					   0,    0,    0,    0,    0,    0,    0,    0,
					   0,    0,    0,    0,    0,    0,    0,    0,
					   0,    0,    0,    0,    0,    0,    0,    0,
					   0,    0,    0,    0,    0,    0,    0,    0,
					   0,    4,    4,    0,    0,    0,    4,  -11
				],
                // queens
				[
					  -1,   14,   14,   16,   16,   14,   14,    1,
					 -10,   20,   30,   45,   60,   25,   30,    1,
					 -20,    5,    5,   50,   45,   25,   20,    1,
					 -10,   20,   20,   45,   55,   40,   40,    5,
					 -14,   20,   20,   45,   35,   30,   30,    5,
					 -16,   -5,   10,    4,    5,   10,    5,    1,
					 -22,  -18,  -18,  -16,   -6,  -18,  -18,  -22,
					 -33,  -22,  -22,  -40,  -10,  -30,  -22,  -40
				],
                // king (looking at endspiel q/k, right-upper corner is preferrable for a mate network)
				[
					 -50,  -30,  -20,  -20,  -10,   -5,   -5,  -30,
					 -10,   15,   15,   16,   16,   30,   20,    5,
					  10,   20,   20,   20,   20,   46,   45,   10,
					  -5,   20,   21,   31,   31,   30,   25,    0,
					 -20,    0,   20,   30,   30,   20,    5,  -10,
					 -20,    0,   10,   20,   20,   10,    5,  -10,
					 -25,  -10,    5,   10,   10,    5,   -5,  -15,
					 -50,  -30,  -30,  -30,  -30,  -30,  -30,  -50
				]
			]
		];

        // king weight should not be LARGE really :D
		// it's for mobility count
        let pieces_weights_const = [
			[  90,  301,  326,  440, 880, 0 ],
			[ 100,  300,  330,  500, 930, 0 ]
		];

		let mobility_base = 8;
		let turn_factor = 4;		// meaning: += self >> 4 or -= self >> 4
		let turn_add_pre = 10;
		let bad_pawn_penalty_pre = [24, 32];
		let good_pawn_reward_pre = [32, 96];
		let outpost_pre = [35, 22]; 
		let bishop_pin_pre = 20;
		let bishop_align_at_king_pre = [14, 4];
		let rook_align_at_king_pre = [12, 6];
		let rook_connected_pre = 6;	// per rook
		let queen_any_battery_pre = 18;
		let queen_strike_possible_pre = 20;
		let knight_seems_promising_pre = 10;

        Self {
            pieces_weights_square_related,
            pieces_weights_const,
			mobility_base,
            turn_factor,
			turn_add_pre,
			bad_pawn_penalty_pre,
			good_pawn_reward_pre,
			outpost_pre,
			bishop_pin_pre,
			bishop_align_at_king_pre,
			rook_align_at_king_pre,
			rook_connected_pre,
			queen_any_battery_pre,
			queen_strike_possible_pre,
			knight_seems_promising_pre
        }
    }
}