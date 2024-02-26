use crate::frame::util::flip;

pub struct Weights {
	/* These weights are stored with respect to the colour, black pieces will provide negative values
		- Usual order is:
		- [ Phase (0-1) ][ Piece (0-11) ][ Square (0-63) ]
		- [ Phase (0-1) ][ Color (0-1) ]
		- [ Color (0-1) ][ Specified ... ] */
	pub pieces:			[[[i32; 64]; 14]; 2],	// add/subtract eval depending on static positional heatmaps in mittielspiel/endspiel
	pub mobility:		   i32,					// score += mobility_base * (mW - mB)
	pub turn:			  [i32;  2],			// works when it's not a 100% draw
	pub turn_fact:		   i32,					// aka mult, but +- bit shift
	pub bad_pawn:		 [[i32;  2];  2],		// isolated, doubled, 1/4 for blocked
	pub good_pawn:		 [[i32;  2];  2],		// passing with possible protection
	pub outpost:		 [[i32;  2];  2],		// for knight/bishop
	pub bpin:			  [i32;  2],			// if bishop is technically pinning smth ; also used to discourage self QK pin possibility
	pub open_lane_rook:	  [i32;  2],			// TEC Rebels - Keep The Lanes Open (for rooks, of course)
	pub random_fact:	   i32,					// for leaf evaluation (it's a tiebreaker, really. recommended value is 3, but 0 is obv. stronger)
}

impl Weights {
	// possible modify by some multipliers
	pub fn init() -> Self {
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
					-15,    5,    5,   25,   31,   10,   10,  -15,
					-20,   -2,    0,   15,   21,    0,   10,  -21,
					-25,   -3,  -10,    0,   -3,  -30,   30,    0,
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
					-15,   -9,    3,   20,   32,   10,  -49,  -20
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
					-20,   30,   20,  -20,   10,    0,   30,   20
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
					-50,  -40,  -30,  -30,  -30,  -30,  -32,  -50
				]
			]
		];

		let pieces_weights_const = [
			[ 100, 311, 326, 540, 930, 0 ],
			[ 100, 310, 330, 550, 920, 0 ]
		];

		let mobility_base = 5;
		let turn_factor = 4; // meaning: += self >> factor or -= self >> factor
		let turn_add_pre = 10;
		let bad_pawn_penalty_pre = [-12, -25];
		let good_pawn_reward_pre = [15, 30];
		let outpost_pre = [18, 12];
		let bishop_pin_pre = 4;
		let open_lane_rook_pre = 12;

		/* Transform PW (flip for white, negative for black) and apply coefficients */

		let mut pieces: [[[i32; 64]; 14]; 2] = [[[0; 64]; 14]; 2];
		for i in 0..2 {
			for j in 0..6 {
				for k in 0..64 {
					pieces[i][(j << 1) + 2][k] =  pieces_weights_square_related[i][j][flip(k)] + pieces_weights_const[i][j];
					pieces[i][(j << 1) + 3][k] = -pieces_weights_square_related[i][j][k      ] - pieces_weights_const[i][j];
				}
			}
		}

		/* Transform other W */
		
		let turn		 	=  [turn_add_pre, -turn_add_pre];
		let bad_pawn	 	= [[bad_pawn_penalty_pre[0], -bad_pawn_penalty_pre[0]], [bad_pawn_penalty_pre[1], -bad_pawn_penalty_pre[1]]];
		let good_pawn		= [[good_pawn_reward_pre[0], -good_pawn_reward_pre[0]], [good_pawn_reward_pre[1], -good_pawn_reward_pre[1]]];
		let outpost			= [[outpost_pre[0], outpost_pre[1]], [-outpost_pre[0], -outpost_pre[1]]];
		let bpin			=  [bishop_pin_pre, -bishop_pin_pre];
		let open_lane_rook	=  [open_lane_rook_pre, -open_lane_rook_pre];
		let random_fact		=  0;

		Self {
			pieces,
			mobility: mobility_base,
			turn,
			turn_fact: turn_factor,
			bad_pawn,
			good_pawn,
			outpost,
			bpin,
			open_lane_rook,
			random_fact,
		}
	}
}