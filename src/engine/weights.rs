use crate::frame::util::flip;

pub struct Weights {
	/* These weights are stored with respect to the colour, black pieces will provide negative values
		- Usual order is:
		- [ Phase (0-1) ][ Piece (0-11) ][ Square (0-63) ]
		- [ Phase (0-1) ][ Color (0-1) ]
		- [ Color (0-1) ][ Specified (if) ... ] */
	pub heatmap:		[[[i32; 64]; 14]; 2],	// add/subtract eval depending on static positional heatmaps in mittielspiel/endspiel
												// it includes pure material weights
												// it also includes some ideas like center control, "pigs on the 7th", etc.
	pub p_isolated:		  [i32;  2],			// double penalty for pawns on semiopen
	pub p_doubled:		  [i32;  2],			// penalty, starts from 2nd pawn
	pub p_phalanga:		  [i32;  2],			// neighboured pawns, starts form 2nd
	pub p_atk_center:	  [i32;  2],			// positional bonus per center square attacked (assym.: d4-e6 for w, d3-e5 for b)
	pub p_outpost:		  [i32;  2],			// pawn produced outpost sq such as protected and couldn't be attacked by other pawn
												// note that sq must be in b6-g7, c5-f7 range.
												// that also means it's not blocked and could be passing, unless ->
	pub p_outpost_block:  [i32;  2], 			// pawn that blockades a strong square (technically preventing pawn above to advance further)
	pub p_semiblocked: 	  [i32;  2],			// pawn blocked on starting square by enemy pieces, use for C/F files
	pub p_blocked:		  [i32;  2],			// pawn blocked on starting square by anything, use for D/E files
	pub p_passing:		 [[i32;  8];  2],		// small additional bonus per passing pawn
	pub nb_outpost:		  [i32;  2],			// knight/bishop stays on outpost sq
	pub nb_outpost_reach: [i32;  2],			// knight/bishop may reach an outpost sq easily
	pub rq_open:		  [i32;  2],			// rook/queen on open file (will apply with atk_open!)
	pub rq_semiopen:	  [i32;  2],			// rook/queen on semiopen file (will apply with atk_semiopen!)
	pub rq_atk_open:	  [i32;  2],			// rook/queen attacks any open file
	pub rq_atk_semiopen:  [i32;  2],			// rook/queen attacks any semiopen file
	pub k_opposition:	 [[i32;  2];  2],		// king has opposition (phased)
	pub k_mobility_as_q: [[i32;  2];  2],		// king security (phased)
	pub k_pawn_dist1:    [[i32;  2];  2],		// bonus if near passing pawn (phased)
	pub k_pawn_dist2:    [[i32;  2];  2],		// bonus if near passing pawn (phased)
	pub k_center_dist1:  [[i32;  2];  2],		// if king near center (phased)
	pub k_center_dist2:  [[i32;  2];  2],		// if king near center (phased)
	pub g_atk_pro:		  [i32;  2],			// per profitable attack (lazy check for pawns)
	pub g_atk_pro_pinned: [i32;  2],			// per profitable attack on pinned piece (lazy check for pawns)
	pub g_atk_pro_double: [i32;  2],			// per double profitable attack (e.g. knight fork!)
	pub g_atk_center:	 [[i32;  2];  2],		// positional bonus per attack on a center square (not like with pawns!) (phased)
	pub g_atk_near_king: [[i32;  5];  2],		// [p, n, b, r, q] attacks intersect with enemy king atk map (lazy check for pawns)
	pub g_atk_ppt:		  [i32;  2],			// per attack on (any colour) passed pawn trajectory
	pub g_ppawn_block:	  [i32;  2],			// passing pawn blocked
	pub g_atk_pro_ppb:	  [i32;  2],			// per profitable attack on passing pawn blocker
	pub s_mobility:		   i32,					// per every square (for N, B, R, Q)
	pub s_bishop_pair:	  [i32;  2],			// bishop pair smol bonus
	pub s_qnight:		  [i32;  2],			// queen & knight smol bonus
	pub s_castled:		 [[i32;  2];  2],
	pub s_turn:			  [i32;  2],
	pub s_turn_div:	       i32,					// score +/-= score / div
	pub rand:			   i32					// random weight of [-rand, +rand] will be added to an evaluated leaf
}

impl Weights {
	// possible modify by some multipliers
	pub fn init() -> Self {

		let pieces_weights_const = [
			[ 328, 1348, 1460, 1908, 4100, 0 ],
			[ 396, 1124, 1188, 2048, 3744, 0 ]
		];

		let p_isolated_pre = -40;
		let p_doubled_pre = -40;
		let p_phalanga_pre = 80;
		let p_atk_center_pre = 40;
		let p_outpost_pre = 80;
		let p_outpost_block_pre = 40;
		let p_semiblocked_pre = -200;
		let p_blocked_pre = -200;
		let p_passing_pre = [0, 120, 140, 160, 180, 200, 200, 0];
		let nb_outpost_pre = 80;
		let nb_outpost_reach_pre = 80;
		let rq_atk_open_pre = 40;
		let rq_atk_semiopen_pre = 20;
		let rq_open_pre = 100;
		let rq_semiopen_pre = 80;
		let k_opposition_pre = [0, 60];
		let k_mobility_as_q_pre = [-5, 0];	// second is always 0
		let k_pawn_dist1_pre = [0, 120];
		let k_pawn_dist2_pre = [0, 40];
		let k_center_dist1_pre = [-40; 120];
		let k_center_dist2_pre = [-40; 80];
		let g_atk_pro_pre = 42;
		let g_atk_pro_pinned_pre = 752 - g_atk_pro_pre;
		let g_atk_pro_double_pre = 1124 - g_atk_pro_pre * 2;
		let g_atk_center_pre = [40, 0];
		let g_atk_near_king_pre = [ 40, 32, 40, 24, 16 ];
		let g_atk_ppt_pre = 20;
		let g_ppawn_block_pre = 40;
		let g_atk_pro_ppb_pre = 40;
		let s_mobility = 5;
		let s_bishop_pair_pre = 80;
		let s_qnight_pre = 40;
		let s_castled_pre = [120, 0];
		let s_turn_pre = 35;
		let s_turn_div = 12;

		/* These are PeSTO values (used as 1/4 score tiebreakers) + Kaissa weights (x4 of course) + my improvisation:
			+54/0 per pawn in center (d4-e6) in mittelspiel
			+20/0 per pawn at c4-c6
			+10/10 per every rank starting from 3rd (e.g. +10/20/30...)
			+54/30 per knight in center (d4-e6)
			-13/0 per knight on g3, d2
			-25/0 per knights and bishops at initial queen squares
			-40/0 per knights and bishops at initial king squares
			-25/0 per rook at b1, g1
			+120/30 per rook on 7th
			+8/0 per queen at initial
		*/

		let pesto = [
			// opening or middlegame
			[
				// pawns
				[
				      0,   0,   0,   0,   0,   0,  0,   0,
 				    148, 184, 111, 145, 118, 176, 84,  39,
				     34,  47,  86, 125, 159,  96, 65,  20,
				     16,  43,  56, 105, 107,  42, 47,   7,
				     -7,  18,  35,  86,  91,  26, 30,  -5,
				    -16,   6,   6,   0,  13,  13, 43,  -2,
				    -35,  -1, -20, -23, -15,  24, 38, -22,
				      0,   0,   0,   0,   0,   0,  0,   0,
				],
				// knights
				[
				    -167, -89, -34, -49,  61, -97, -15, -107,
				     -73, -41,  72,  36,  23,  62,   7,  -17,
				     -47,  60,  37, 119, 138, 129,  73,   44,
				      -9,  17,  19, 107,  91,  69,  18,   22,
				     -13,   4,  16,  67,  82,  19,  21,   -8,
				     -23,  -9,  12,  10,  19,  17,  12,  -16,
				     -29, -53, -12, -16,  -1,  18, -14,  -19,
				    -105, -46, -58, -33, -17, -28, -89, -23,
				],
				// bishops
				[
				    -29,   4, -82, -37, -25, -42,   7,  -8,
				    -26,  16, -18, -13,  30,  59,  18, -47,
				    -16,  37,  43,  40,  35,  50,  37,  -2,
				     -4,   5,  19,  50,  37,  37,   7,  -2,
				     -6,  13,  13,  26,  34,  12,  10,   4,
				      0,  15,  15,  15,  14,  27,  18,  10,
				      4,  15,  16,   0,   7,  21,  33,   1,
				    -33,  -3, -39, -21, -13, -52, -39, -21,
				],
				// rooks
				[
				     32,  42,  32,  51,  63,   9,  31,  43,
				    147, 152, 178, 182, 200, 187, 146, 164,
				     -5,  19,  26,  36,  17,  45,  61,  16,
				    -24, -11,   7,  26,  24,  35,  -8, -20,
				    -36, -26, -12,  -1,   9,  -7,   6, -23,
 				    -45, -25, -16, -17,   3,   0,  -5, -33,
 				    -44, -16, -20,  -9,  -1,  11,  -6, -71,
 				    -19, -38,   1,  17,  16,   7, -62, -26,
				],
				// queens
				[
				    -28,   0,  29,  12,  59,  44,  43,  45,
				    -24, -39,  -5,   1, -16,  57,  28,  54,
				    -13, -17,   7,   8,  29,  56,  47,  57,
				    -27, -27, -16, -16,  -1,  17,  -2,   1,
 				     -9, -26,  -9, -10,  -2,  -4,   3,  -3,
 				    -14,   2, -11,  -2,  -5,   2,  14,   5,
				    -35,  -8,  11,   2,   8,  15,  -3,   1,
 				     -1, -18,  -9,  18, -15, -25, -31, -50,
				],
				// king
				[
				    -65,  23,  16, -15, -56, -34,   2,  13,
				     29,  -1, -20,  -7,  -8,  -4, -38, -29,
				     -9,  24,   2, -16, -20,   6,  22, -22,
				    -17, -20, -12, -27, -30, -25, -14, -36,
				    -49,  -1, -27, -39, -46, -44, -33, -51,
				    -14, -14, -22, -46, -44, -30, -15, -27,
				      1,   7,  -8, -64, -43, -16,   9,   8,
				    -15,  36,  12, -54,   8, -28,  24,  14,
				]
			],
			// endgame
			[
				// pawns
				[
					  0,   0,   0,   0,   0,   0,   0,   0,
					228, 223, 208, 184, 197, 182, 215, 237,
					134, 140, 125, 107,  96,  93, 122, 124,
					 62,  54,  43,  35,  28,  34,  47,  47,
					 33,  29,  17,  13,  13,  12,  23,  19,
					 14,  17,   4,  11,  10,   5,   9,   2,
					 13,   8,   8,  10,  13,   0,   2,  -7,
					  0,   0,   0,   0,   0,   0,   0,   0,
				],
				// knights
				[
					-58, -38, -13, -28, -31, -27, -63, -99,
					-25,  -8, -25,  -2,  -9, -25, -24, -52,
					-24, -20,  10,  39,  29,  -9, -19, -41,
					-17,   3,  22,  52,  52,  11,   8, -18,
					-18,  -6,  16,  55,  46,  17,   4, -18,
					-23,  -3,  -1,  15,  10,  -3, -20, -22,
					-42, -20, -10,  -5,  -2, -20, -23, -44,
					-29, -51, -23, -15, -22, -18, -50, -64,
				],
				// bishops
				[
					-14, -21, -11,  -8, -7,  -9, -17, -24,
					 -8,  -4,   7, -12, -3, -13,  -4, -14,
					  2,  -8,   0,  -1, -2,   6,   0,   4,
					 -3,   9,  12,   9, 14,  10,   3,   2,
					 -6,   3,  13,  19,  7,  10,  -3,  -9,
					-12,  -3,   8,  10, 13,   3,  -7, -15,
					-14, -18,  -7,  -1,  4,  -9, -15, -27,
					-23,  -9, -23,  -5, -9, -16,  -5, -17,
				],
				// rooks
				[
					13, 10, 18, 15, 12,  12,   8,   5,
					41, 43, 43, 41, 27,  33,  38,  33,
					 7,  7,  7,  5,  4,  -3,  -5,  -3,
					 4,  3, 13,  1,  2,   1,  -1,   2,
					 3,  5,  8,  4, -5,  -6,  -8, -11,
					-4,  0, -5, -1, -7, -12,  -8, -16,
					-6, -6,  0,  2, -9,  -9, -11,  -3,
					-9,  2,  3, -1, -5, -13,   4, -20,
				],
				// queens
				[
					 -9,  22,  22,  27,  27,  19,  10,  20,
					-17,  20,  32,  41,  58,  25,  30,   0,
					-20,   6,   9,  49,  47,  35,  19,   9,
					  3,  22,  24,  45,  57,  40,  57,  36,
					-18,  28,  19,  47,  31,  34,  39,  23,
					-16, -27,  15,   6,   9,  17,  10,   5,
					-22, -23, -30, -16, -16, -23, -36, -32,
					-33, -28, -22, -43,  -5, -32, -20, -41,
				],
				// king
				[
					-74, -35, -18, -18, -11,  15,   4, -17,
					-12,  17,  14,  17,  17,  38,  23,  11,
					 10,  17,  23,  15,  20,  45,  44,  13,
					 -8,  22,  24,  27,  26,  33,  26,   3,
					-18,  -4,  21,  24,  27,  23,   9, -11,
					-19,  -3,  11,  21,  23,  16,   7,  -9,
					-27, -11,   4,  13,  14,   4,  -5, -17,
					-53, -34, -21, -11, -28, -14, -24, -43
				]
			]
		];

		/* Transform PW (flip for white, negative for black) and apply coefficients */

		let mut heatmap: [[[i32; 64]; 14]; 2] = [[[0; 64]; 14]; 2];
		for i in 0..2 {
			for j in 0..6 {
				for k in 0..64 {
					heatmap[i][(j << 1) + 2][k] =  pesto[i][j][flip(k)] * 3 / 2 + pieces_weights_const[i][j];
					heatmap[i][(j << 1) + 3][k] = -pesto[i][j][k      ] * 3 / 2 - pieces_weights_const[i][j];
				}
			}
		}

		/* Transform other W */
	
		let mut g_atk_near_king: [[i32; 5]; 2] = [g_atk_near_king_pre, g_atk_near_king_pre];
		for i in 0..5 {
			g_atk_near_king[1][i] = -g_atk_near_king[1][i];
		}

		let mut p_passing: [[i32; 8]; 2] = [p_passing_pre, p_passing_pre];
		for i in 0..4 {
			p_passing[1].swap(i, 7 - i);
		}
		for i in 0..8 {
			p_passing[1][i] = -p_passing[1][i];
		}

		Self {
			heatmap,
			p_isolated: colour_transform(p_isolated_pre),
			p_doubled: colour_transform(p_doubled_pre),
			p_phalanga: colour_transform(p_phalanga_pre),
			p_atk_center: colour_transform(p_atk_center_pre),
			p_outpost: colour_transform(p_outpost_pre),
			p_outpost_block: colour_transform(p_outpost_block_pre),
			p_semiblocked: colour_transform(p_semiblocked_pre),
			p_blocked: colour_transform(p_blocked_pre),
			p_passing,
			nb_outpost: colour_transform(nb_outpost_pre),
			nb_outpost_reach: colour_transform(nb_outpost_reach_pre),
			rq_open: colour_transform(rq_open_pre),
			rq_semiopen: colour_transform(rq_semiopen_pre),
			rq_atk_open: colour_transform(rq_atk_open_pre),
			rq_atk_semiopen: colour_transform(rq_atk_semiopen_pre),
			k_opposition: [colour_transform(k_opposition_pre[0]), colour_transform(k_opposition_pre[1])],
			k_mobility_as_q: [colour_transform(k_mobility_as_q_pre[0]), colour_transform(k_mobility_as_q_pre[1])],
			k_pawn_dist1: [colour_transform(k_pawn_dist1_pre[0]), colour_transform(k_pawn_dist1_pre[1])],
			k_pawn_dist2: [colour_transform(k_pawn_dist2_pre[0]), colour_transform(k_pawn_dist2_pre[1])],
			k_center_dist1: [colour_transform(k_center_dist1_pre[0]), colour_transform(k_center_dist1_pre[1])],
			k_center_dist2: [colour_transform(k_center_dist2_pre[0]), colour_transform(k_center_dist2_pre[1])],
			g_atk_pro: colour_transform(g_atk_pro_pre),
			g_atk_pro_pinned: colour_transform(g_atk_pro_pinned_pre),
			g_atk_pro_double: colour_transform(g_atk_pro_double_pre),
			g_atk_center: [colour_transform(g_atk_center_pre[0]), colour_transform(g_atk_center_pre[1])],
			g_atk_near_king,
			g_atk_ppt: colour_transform(g_atk_ppt_pre),
			g_ppawn_block: colour_transform(g_ppawn_block_pre),
			g_atk_pro_ppb: colour_transform(g_atk_pro_ppb_pre),
			s_mobility,
			s_bishop_pair: colour_transform(s_bishop_pair_pre),
			s_qnight: colour_transform(s_qnight_pre),
			s_castled: [colour_transform(s_castled_pre[0]), colour_transform(s_castled_pre[1])],
			s_turn: colour_transform(s_turn_pre),
			s_turn_div,
			rand: 0
		}
	}
}

fn colour_transform(weight: i32) -> [i32; 2] {
	[weight, -weight]
}