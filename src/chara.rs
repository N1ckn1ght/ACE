// The main module of the chess engine.
// The board its heart, the search its muscles, this is its brain.
// ANY changes to the board MUST be done through the character's methods!

use std::{collections::{HashMap, HashSet}, cmp::{min, max}};
use rand::{rngs::ThreadRng, Rng};
use crate::{util::*, board::Board, gen::Zobrist, engine::search};

// TODO: use f16 instead of f32 to cache approx ~40 mil./2 GiB more positions?
// TODO: use i16 instead of floats for faster add/sub operations?
pub struct Chara {
	/* These weights are stored with respect to the colour, black pieces will provide negative values
		- Usual order is:
		- [ Game Phase (0-1) ][ Piece (0-11) ][ Square (0-63) ]	*/
	pub pieces_weights:		[[[f32; 64]; 12]; 2],	// add/subtract eval depending on static positional heatmaps in mittielspiel/endspiel
	pub mobility_weights:	[[[f32; 32]; 12]; 2],	// add/subtract eval depending on piece mobility in mittlespiel/endspiel
	pub turn_weights:		[[f32; 2]; 2],			// mult/add weights per turn if not a 100% draw
	pub align_weights:		[[[f32; 6]; 2]; 2],		// add weight per b/r/q pieces aligned (x-ray) against king / against near square in mittelspiel/endspiel
													// 		for straight aligns it checks if the lane is open (no ally pawn in the way)
	pub battery_weights:	[[f32; 4]; 2],			// add weight per batteries: b+q / r+r horizontal / r+r vertical / r+q vertical
													//		note: in case of b+q, r+q, the weight is added per queen; for rooks it's doubled
	// pub rook_lane_open:	f32,					// add weight, small self-explanatory bonus (obsolete for now because of crazy mobility bonus?)
	/* TODO: Better pawn structure evaluation (hash them separately) */
	pub dp_weight:			f32,					// mult weight per pawn if it has other pawns of same color at this file (use as penalty)
	pub pp_weights:			[f32; 3],				// mult weight per passing / protected by pawn / passing + protected by pawn (unique bonus)
	pub outpost_weights:	[[f32; 2]; 2],			// add weight per passing knight / bishop that's defended by pawn
	pub dan_possible:		[[f32; 3]; 2],			// add weight per knight if it is able to check / able to royal fork / able to fork heavy pieces
	pub random_range:		f32,					// +-(0 - random_range) value per evaluated position
	/* Temporary cache; meaning: stores evals and self-cleans when no RAM :( */
	pub cache:				HashMap<u64, Eval>,		// evaluated position stored here (RIP RAM)
	/* Permanent cache; meaning: only already made moves, will take back reverted ones as well */
	pub cache_perm_vec:		Vec<u64>,				// previous board hashes stored here to call more quick hash_iter() function
	pub cache_perm_set:		HashSet<u64>,			// for fast checking if this position had occured before in this line
													// note: it's always 1 hash behind
	/* Accessible constants */
	pub zobrist:			Zobrist,
	pub rng:				ThreadRng
}

/* For more understandable indexing in eval() */
const TURN_MULT:		usize = 0;
const TURN_ADD:			usize = 1;
const ALIGN_BISHOP:		usize = 0;
const ALIGN_ROOK:		usize = 1;
const ALIGN_QUEEN:		usize = 2;
const ALIGN_NEAR:		usize = 3;
const BATTERY_BQ:		usize = 0;
const BATTERY_RRH:		usize = 1;
const BATTERY_RRV:		usize = 2;
const BATTERY_RQV:		usize = 3;
const PAWN_PASSING:		usize = 0;
const PAWN_PROTECTED:	usize = 1;
const PAWN_COMBINED:	usize = 2;
const OUTPOST_KNIGHT:	usize = 0;
const OUTPOST_BISHOP:	usize = 1;
const DAN_CHECK:		usize = 0;
const DAN_ROYAL_FORK:	usize = 1;
const DAN_FORK:			usize = 2;

impl Chara {
	// It's a little messy, but any evals are easily modifiable.
	// No need in modifying something twice to be applied for the different coloir, neither there's a need to write something twice in eval()
    pub fn init(board: &Board, random_range: f32, aggressiveness: f32) -> Chara {
		let zobrist = Zobrist::default();

		// let piece_wmult: f32					= 1.5;	-- Result were somewhat better
		let piece_wmult: f32					= 1.2;
		let piece_square_related_wmult: f32		= 0.75 / (aggressiveness * 0.5  + 0.5 );	// really slight balance-out
		let mobility_wmult: f32					= 1.0  * (aggressiveness * 0.5  + 0.5 );
		let align_wmult: f32					= 1.0  *  aggressiveness;
		let battery_wmult: f32					= 1.0  *  aggressiveness;
		let pawn_structure_wmult: f32			= 0.5  * (aggressiveness * 0.5  + 0.5 );
		let minor_piece_pos_wmult: f32			= 1.0  * (aggressiveness * 0.5  + 0.5 );

		/* 2-staged static piece positional weights
			- they will be tuned, for now I need to focus no search */
		/* Inspiration for a current implementation was taken from:
			- Simplified Evaluation Function by Tomasz Michniewski
			- PeSTO's Evaluation Function by Tom Kerrigan, Pawel Koziol, Ronald Friederich (The OG!!)
			Note: some modifications are ill-intended for fun sake, copying is not advised :D */
		// TODO: tune them, add more dynamic types of weights
		// TODO (optional): store weights in file maybe to possibly autotune them later
		let pieces_weights_square_related = [
			// opening or middlegame
			[
                // pawns
				[
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					 0.96,  1.21,  0.7 ,  0.95,  0.71,  1.2 ,  0.6 ,  0.0 ,
					-0.05,  0.05,  0.25,  0.3 ,  0.3 ,  0.5 ,  0.25,  0.1 ,
					-0.15,  0.05,  0.05,  0.2 ,  0.25,  0.1 ,  0.1 , -0.15,
					-0.2 , -0.02,  0.0 ,  0.15,  0.2 ,  0.0 ,  0.1 , -0.2 ,
					-0.25, -0.03, -0.1 ,  0.0 ,  0.0 , -0.3 ,  0.3 ,  0.0 ,
					-0.3 , -0.01, -0.15, -0.15, -0.15,  0.3 ,  0.3 , -0.15,
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0
				],
                // knights
				[
					-1.63, -0.9 , -0.3 , -0.49,  0.01, -0.8 , -0.2 , -1.03,
					-0.72, -0.4 ,  0.71,  0.35,  0.2 ,  0.6 ,  0.01, -0.05,
					-0.4 ,  0.6 ,  0.4 ,  0.65,  0.85,  1.3 ,  0.72,  0.4 ,
					-0.05,  0.2 ,  0.2 ,  0.54,  0.4 ,  0.7 ,  0.22,  0.2 ,
					-0.1 ,  0.0 ,  0.15,  0.21,  0.21,  0.2 ,  0.2 , -0.02,
					-0.3 , -0.08,  0.15,  0.1 ,  0.15,  0.2 ,  0.25, -0.15,
					-0.3 , -0.5 , -0.15,  0.01,  0.01,  0.0 , -0.1 , -0.2 ,
					-1.1 , -0.5 , -0.6 , -0.1 , -0.1 , -0.02, -0.5 , -0.25
				],
                // bishops
				[
					-0.3 , -0.65, -0.5 , -0.35, -0.25, -0.3 , -0.6 , -0.1 ,
					-0.25,  0.2 , -0.15, -0.05,  0.3 ,  0.6 ,  0.2 , -0.4 ,
					-0.1 ,  0.3 ,  0.4 ,  0.4 ,  0.35,  0.5 ,  0.37,  0.0 ,
					 0.0 ,  0.15,  0.11,  0.5 ,  0.37,  0.37,  0.15,  0.0 ,
					 0.0 ,  0.1 ,  0.3 ,  0.2 ,  0.2 ,  0.12,  0.1 ,  0.0 ,
					 0.02,  0.15,  0.15,  0.05,  0.0 ,  0.0 ,  0.05,  0.06,
					 0.0 ,  0.15,  0.05,  0.0 ,  0.1 ,  0.15,  0.3 ,  0.0 ,
					-0.3 ,  0.0 , -0.15, -0.15, -0.15, -0.1 , -0.4 , -0.25
				],
                // rooks
				[
					 0.3 ,  0.4 ,  0.3 ,  0.5 ,  0.6 ,  0.1 ,  0.3 ,  0.4 ,
					 0.3 ,  0.3 ,  0.6 ,  0.6 ,  0.8 ,  0.6 ,  0.3 ,  0.4 ,
					-0.05,  0.15,  0.2 ,  0.35,  0.2 ,  0.45,  0.5 ,  0.13,
					-0.25, -0.1 ,  0.05,  0.2 ,  0.2 ,  0.25, -0.05, -0.2 ,
					-0.3 , -0.2 , -0.05,  0.0 ,  0.05, -0.05,  0.05, -0.25,
					-0.46, -0.2 , -0.15, -0.1 ,  0.05,  0.0 , -0.05, -0.3 ,
					-0.4 , -0.12, -0.13, -0.01,  0.0 ,  0.1 , -0.07, -0.5 ,
					-0.15, -0.09,  0.03,  0.2 ,  0.3 ,  0.07, -0.39, -0.2
				],
                // queens
				[
					-0.29,  0.0 ,  0.3 ,  0.11,  0.6 ,  0.44,  0.43,  0.45,
					-0.22, -0.4 , -0.01,  0.01, -0.15,  0.57,  0.25,  0.5 ,
					-0.15, -0.15,  0.05,  0.05,  0.3 ,  0.55,  0.47,  0.56,
					-0.25, -0.25, -0.15, -0.15,  0.0 ,  0.15,  0.0 ,  0.0 ,
					-0.05, -0.25, -0.1 , -0.1 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					-0.1 ,  0.05, -0.1 , -0.01, -0.04,  0.0 ,  0.11,  0.01,
					-0.3 ,  0.0 ,  0.11,  0.02,  0.07,  0.05, -0.01,  0.0 ,
					-0.1 , -0.2 , -0.02,  0.1 , -0.15, -0.25, -0.3 , -0.5
				],
                // king
				[
					-0.4 , -0.4 , -0.4 , -0.4 , -0.1 , -0.4 , -0.4 , -0.35,
					-0.35,  0.4 , -0.4 , -0.4 , -0.4 , -0.4 ,  0.35, -0.3 ,
					-0.3 , -0.35, -0.4 , -0.4 , -0.4 , -0.35, -0.3 , -0.2 ,
					-0.25, -0.3 , -0.35, -0.4 , -0.4 , -0.3 , -0.2 , -0.3 ,
					-0.2 , -0.25, -0.3 , -0.35, -0.35, -0.2 , -0.25, -0.2 ,
					-0.15, -0.2 , -0.25, -0.3 , -0.05, -0.3 , -0.2 , -0.1 ,
					 0.0 ,  0.01, -0.05, -0.25, -0.1 , -0.1 ,  0.03,  0.02,
					-0.2 ,  0.3 ,  0.2 , -0.2 ,  0.1 , -0.1 ,  0.3 ,  0.2
				]
            ],
			// endgame
			[
                // pawns
				[
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					 1.95,  1.8 ,  1.6 ,  1.4 ,  1.5 ,  1.3 ,  1.65,  1.9 ,
					 0.95,  1.0 ,  0.9 ,  0.7 ,  0.55,  0.5 ,  0.8 ,  0.85,
					 0.3 ,  0.2 ,  0.19,  0.05,  0.03,  0.04,  0.2 ,  0.2 ,
					 0.15, -0.1 , -0.04, -0.05, -0.05, -0.1 ,  0.0 , -0.01,
					 0.0 ,  0.05, -0.05,  0.0 ,  0.0 , -0.05,  0.0 , -0.1 ,
					-0.2 , -0.2 , -0.2 , -0.3 , -0.3 , -0.01, -0.2 , -0.3 ,
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0
				],
                // knights
				[
					-0.6 , -0.45, -0.15, -0.3 , -0.3 , -0.3 , -0.45, -0.99,
					-0.25, -0.2 , -0.1 ,  0.0 ,  0.0 , -0.1 , -0.2 , -0.45,
					-0.2 , -0.1 ,  0.05,  0.1 ,  0.1 ,  0.01, -0.1 , -0.2 ,
					-0.2 ,  0.05,  0.15,  0.2 ,  0.2 ,  0.1 ,  0.05, -0.2 ,
					-0.2 ,  0.0 ,  0.1 ,  0.2 ,  0.2 ,  0.1 ,  0.0 , -0.2 ,
					-0.2 , -0.1 ,  0.0 ,  0.1 ,  0.1 ,  0.0 , -0.2 , -0.2 ,
					-0.45, -0.2 , -0.2 , -0.05, -0.05, -0.2 , -0.2 , -0.45,
					-0.7 , -0.45, -0.3 , -0.3 , -0.3 , -0.3 , -0.45, -0.7
				],
				// bishops
				[
					-0.3 , -0.08, -0.05, -0.05, -0.05, -0.05, -0.08, -0.3 ,
					-0.08, -0.01,  0.0 , -0.04, -0.03,  0.0 , -0.01, -0.08,
					 0.05,  0.0 ,  0.05,  0.0 ,  0.0 ,  0.06,  0.0 ,  0.05,
					 0.0 ,  0.06,  0.1 ,  0.05,  0.06,  0.1 ,  0.03,  0.0 ,
					 0.0 ,  0.05,  0.1 ,  0.11,  0.05,  0.1 ,  0.0 ,  0.0 ,
					-0.05,  0.0 ,  0.06,  0.04,  0.04,  0.01,  0.0 , -0.05,
					-0.15, -0.1 ,  0.0 ,  0.0 ,  0.0 ,  0.0 , -0.1 , -0.15,
					-0.25, -0.15, -0.2 , -0.1 , -0.1 , -0.2 , -0.15, -0.25
				],
				// rooks (it kinda makes sense)
				[
					 0.1 ,  0.1 ,  0.1 ,  0.1 ,  0.1 ,  0.1 ,  0.0 ,  0.0 ,
					 0.1 ,  0.1 ,  0.1 ,  0.1 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
					 0.0 ,  0.04,  0.04,  0.0 ,  0.0 ,  0.0 ,  0.04, -0.11
				],
                // queens
				[
					-0.01,  0.14,  0.14,  0.16,  0.16,  0.14,  0.14,  0.01,
					-0.1 ,  0.2 ,  0.3 ,  0.45,  0.6 ,  0.25,  0.3 ,  0.01,
					-0.2 ,  0.05,  0.05,  0.5 ,  0.45,  0.25,  0.2 ,  0.01,
					-0.1 ,  0.2 ,  0.2 ,  0.45,  0.55,  0.4 ,  0.4 ,  0.05,
					-0.14,  0.2 ,  0.2 ,  0.45,  0.35,  0.3 ,  0.3 ,  0.05,
					-0.16, -0.05,  0.1 ,  0.04,  0.05,  0.1 ,  0.05,  0.01,
					-0.22, -0.18, -0.18, -0.16, -0.06, -0.18, -0.18, -0.22,
					-0.33, -0.22, -0.22, -0.4 , -0.1 , -0.3 , -0.22, -0.4
				],
                // king
				[
					-0.5 , -0.3 , -0.2 , -0.2 , -0.1 , -0.05, -0.05, -0.3 ,
					-0.1 ,  0.15,  0.15,  0.16,  0.16,  0.3 ,  0.2 ,  0.05,
					 0.1 ,  0.2 ,  0.2 ,  0.2 ,  0.2 ,  0.46,  0.45,  0.1 ,
					-0.05,  0.2 ,  0.21,  0.31,  0.31,  0.3 ,  0.25,  0.0 ,
					-0.2 ,  0.0 ,  0.2 ,  0.30,  0.30,  0.2 ,  0.05, -0.1 ,
					-0.2 ,  0.0 ,  0.1 ,  0.2 ,  0.2 ,  0.1 ,  0.05, -0.1 ,
					-0.25, -0.1 ,  0.05,  0.1 ,  0.1 ,  0.05, -0.05, -0.15,
					-0.5 , -0.3 , -0.3 , -0.3 , -0.3 , -0.3 , -0.3 , -0.5
				]
			]
		];
		// 0 for a king seems sus but we don't really care bc we don't calc its capture
		let pieces_weights_const = [
			[ 0.85,  3.3 ,  3.0 ,  4.0 ,  9.0 ,  0.0 ],
			[ 0.9 ,  3.0 ,  3.1 ,  5.0 ,  9.0 ,  0.0 ]
		];
		// transform (flip for white, negative for black) and apply coefficients
		let mut pieces_weights = [[[0.0; 64]; 12]; 2];
		for i in 0..2 {
			for j in 0..6 {
				for k in 0..64 {
					pieces_weights[i][ j << 1     ][k] =  pieces_weights_square_related[i][j][flip(k)] * piece_square_related_wmult + pieces_weights_const[i][j] * piece_wmult;
					pieces_weights[i][(j << 1) | 1][k] = -pieces_weights_square_related[i][j][k      ] * piece_square_related_wmult - pieces_weights_const[i][j] * piece_wmult;
				}
			}
		}

		/* Mobility weights
			They are dynamic and thus are more general, however they will overlap with static weights 
			Maybe it's good to test different coefficient values for them or even tweak as standalone weights */
		let mobility_weights_const = [
			// knight
			[
				-1.7 , -1.0 , -0.3 , -0.07,  0.0 ,  0.2 ,  0.35,  0.4 ,
				 0.4 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0
			],
			// bishop
			[
				-1.7 , -1.2 , -0.5 , -0.3 ,  0.0 ,  0.21,  0.36,  0.45,
				 0.5 ,  0.5 ,  0.5 ,  0.5 ,  0.5 ,  0.5 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0
			],
			// rook
			[
				-1.7 , -1.2 , -1.04, -0.3 , -0.15,  0.0 ,  0.15,  0.25,
				 0.35,  0.45,  0.5 ,  0.55,  0.6 ,  0.6 ,  0.6 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0
			],
			// queen
			[
				-2.1 , -1.35, -0.65, -0.3 , -0.2 ,  0.0 ,  0.15,  0.3 ,
				 0.4 ,  0.5 ,  0.6 ,  0.7 ,  0.8 ,  0.85,  0.9 ,  0.95,
				 1.0 ,  1.0 ,  1.0 ,  1.0 ,  1.0 ,  1.0 ,  1.0 ,  1.0 ,
				 1.0 ,  1.0 ,  1.0 ,  1.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0
			],
			// king (opening/mittelspiel)
			[
				 0.0 ,  0.05,  0.1 ,  0.1 , -0.6 , -0.6 , -0.6 , -0.6 ,
				-0.6 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0
			],
			// king (endspiel)
			[
				-1.31, -0.5 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,
				 0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0 ,  0.0
			]
		];
		// transform and apply coefficients
		let mut mobility_weights = [[[0.0; 32]; 12]; 2];
		for j in 0..5 {
			for k in 0..32 {
				for i in 0..2 {
					mobility_weights[i][ (j + 1) << 1     ][k] =  mobility_weights_const[j][k] * mobility_wmult;
					mobility_weights[i][((j + 1) << 1) | 1][k] = -mobility_weights_const[j][k] * mobility_wmult;
				}
			}
		}
		for k in 0..32 {
			mobility_weights[1][10][k] =  mobility_weights_const[5][k] * mobility_wmult;
			mobility_weights[1][11][k] = -mobility_weights_const[5][k] * mobility_wmult;
		}

		/* More dynamic weights */
		let mut turn_weights_pre	=  [ 1.15,  0.1 ];
		let mut align_weights_pre   = [[ 0.35,  0.3 ,  0.3 ,  0.21,  0.15,  0.3 ], [ 0.2 ,  0.2 ,  0.1 ,  0.05,  0.1,  0.1 ]];
		let mut battery_weights_pre =  [ 0.21,  0.22,  0.45,  0.11];
		let mut dp_weight			=    0.75;
		let mut pp_weights			=  [ 1.21,  1.11,  1.42];
		let mut outpost_weight_pre  =  [ 0.4 ,  0.2 ];
		let mut dan_possible_pre    =  [ 0.4 ,  0.6 ,  0.3 ];

		/* Apply coefficients */
		turn_weights_pre.iter_mut().for_each(|w| *w = *w * mobility_wmult);
		align_weights_pre.iter_mut().for_each(|arr| arr.iter_mut().for_each(|w| *w = *w * align_wmult));
		battery_weights_pre.iter_mut().for_each(|w| *w = *w * battery_wmult);
		dp_weight += -0.25 + (-pawn_structure_wmult).exp2() * 0.5;
		pp_weights.iter_mut().for_each(|w| *w = *w * pawn_structure_wmult);
		outpost_weight_pre.iter_mut().for_each(|w| *w = *w * minor_piece_pos_wmult);
		dan_possible_pre.iter_mut().for_each(|w| *w = *w * minor_piece_pos_wmult);

		/* Transform */
		let mut turn_weights 	= [turn_weights_pre.clone(); 2];
		turn_weights[1][1] = -turn_weights[0][1];
		turn_weights[1][0] = 1.0 / turn_weights[0][0];
		let mut battery_weights = [battery_weights_pre.clone(); 2];
		battery_weights[1].iter_mut().for_each(|w| *w = *w * -1.0);
		let mut outpost_weights	= [outpost_weight_pre.clone(); 2];
		outpost_weights[1].iter_mut().for_each(|w| *w = *w * -1.0);
		let mut dan_possible	= [dan_possible_pre.clone(); 2];
		dan_possible[1].iter_mut().for_each(|w| *w = *w * -1.0);
		let mut align_weights 	= [[[0.0; 6]; 2]; 2];
		for i in 0..2 {
			for k in 0..6 {
				align_weights[i][0][k] =  align_weights_pre[i][k];
				align_weights[i][1][k] = -align_weights_pre[i][k];
			}
		}

		let mut cache_perm_vec = Vec::with_capacity(300);
		cache_perm_vec.push(zobrist.cache_new(board));

		Self {
			pieces_weights,
			mobility_weights,
			turn_weights,
			align_weights,
			battery_weights,
			dp_weight,
			pp_weights,
			outpost_weights,
			dan_possible,
			random_range,
			cache:			HashMap::default(),
			cache_perm_vec,
			cache_perm_set: HashSet::default(),
			zobrist,
			rng:			rand::thread_rng()
		}
    }

	// Somewhat of a 0 level of the search(), but a necessary one
	pub fn think(&mut self, board: &mut Board, depth: i16, last_eval: Eval) -> Vec<EvalMove> {
		let mut moves = board.get_legal_moves();
		if moves.len() == 0 {
			return vec![];
		}
		moves.sort();
		let mut moves_evaluated = vec![];
		let maximize = !board.turn;	// 0 -> white to move -> maximize

		let mut alpha = f32::max(0.0, last_eval.score + 2.0);
		let mut beta  = f32::min(0.0, last_eval.score - 2.0);

		if maximize {
			for mov in moves.into_iter() {
				self.make_move(board, mov);
				let temp = search(self, board, alpha, beta, false, board.no + depth - 1, false, mov);
				self.revert_move(board);
				moves_evaluated.push(EvalMove::new(mov, temp));
				// QUIT W if time out!
				alpha = max(alpha, temp);
				if alpha >= beta {
					break;
				}
			}
		} else {
			for mov in moves.into_iter() {
				self.make_move(board, mov);
				let temp = search(self, board, alpha, beta, true , board.no + depth - 1, false, mov);
				self.revert_move(board);
				moves_evaluated.push(EvalMove::new(mov, temp));
				// QUIT W if time out!
				beta = min(beta, temp);
				if beta <= alpha {
					break;
				}
			}
		}

		moves_evaluated.sort_by(|a: &EvalMove, b: &EvalMove| a.eval.cmp(&b.eval));
		if !board.turn {
			moves_evaluated.reverse();
		}
		//	moves_evaluated.iter_mut().for_each(|em|);	- TODO: fix mate counter
		moves_evaluated
	}

	pub fn make_move(&mut self, board: &mut Board, mov: u64) {
		let prev_hash = *self.cache_perm_vec.last().unwrap();
		self.cache_perm_set.insert(prev_hash);
		board.make_move(mov);
		let hash = self.zobrist.cache_iter(board, mov, prev_hash);
		self.cache_perm_vec.push(hash);
	}
	
	// pub fn make_move_by_hash(&mut self, board: &mut Board, mov: u64, hash: u64) { }

	pub fn revert_move(&mut self, board: &mut Board) {
		board.revert_move();
		self.cache_perm_vec.pop();
		self.cache_perm_set.remove(&self.cache_perm_vec.last().unwrap());
	}

	/* Warning! 
		1) Search MUST use check extension! Eval does NOT evaluate checks!
		2) Search MUST determine if the game ended! Eval should NOT evaluate staled/mated positions specifically!
	 */
	pub fn eval(&mut self, board: &Board) -> Eval {
		let hash = *self.cache_perm_vec.last().unwrap();
		if self.cache.contains_key(&hash) {
			return self.cache[&hash];
		}

		/* CHECK FOR ANISH GIRI,
			SETUP SCORE APPLICATION */

		if board.hmc == 50 || self.cache_perm_set.contains(&hash) {
			return Eval::new(0.0, 0, board.no);
		}

		let counter = (board.bbs[N] | board.bbs[N2]).count_ones() * 3 + 
					  (board.bbs[B] | board.bbs[B2]).count_ones() * 3 + 
					  (board.bbs[R] | board.bbs[R2]).count_ones() * 4 +
					  (board.bbs[Q] | board.bbs[Q2]).count_ones() * 8;

		if counter < 4 && board.bbs[P] | board.bbs[P2] == 0 {
			self.cache.insert(hash, Eval::new(0.0, 0, board.no));
			return Eval::new(0.0, 0, board.no);
		}

		let phase = (counter < 31) as usize;

		let mut score: f32 = 0.0;
		let sides = [board.get_occupancies(false), board.get_occupancies(true)];
		let occup = sides[0] | sides[1];
		let kbits = [gtz(board.bbs[K]), gtz(board.bbs[K2])];

		/* SCORE APPLICATION BEGIN */

		let pawns = [board.bbs[P], board.bbs[P2]];
		for (ally, mut bb) in pawns.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				let mut ts = self.pieces_weights[phase][P | ally][csq];
				/* Mult penalty per multiplied pawn on the same file */
				if board.maps.files[csq] & board.bbs[P | ally] != 0 {
					ts *= self.dp_weight;
				}
				/* Mult bonus per passing and/or easily protected pawns */
				if board.is_passing(csq, board.bbs[P | enemy], ally) {
					if board.is_easily_protected(csq, board.bbs[P | ally], occup, ally, enemy) {
						ts *= self.pp_weights[PAWN_COMBINED];
					} else {
						ts *= self.pp_weights[PAWN_PASSING];
					}
				} else if board.is_easily_protected(csq, board.bbs[P | ally], occup, ally, enemy) {
					ts *= self.pp_weights[PAWN_PROTECTED];
				}
				score += ts;
			}
		}

		let knights = [board.bbs[N], board.bbs[N2]];
		for (ally, mut bb) in knights.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.pieces_weights[phase][N | ally][csq];
				score += self.mobility_weights[phase][N | ally][(board.maps.attacks_knight[csq] & !sides[ally]).count_ones() as usize];
				/* Add score if it's an outpost */
				if board.is_outpost(csq, board.bbs[P | ally], board.bbs[P | enemy], ally != 0) {
					score += self.outpost_weights[ally][OUTPOST_KNIGHT];
				}
				/* Add score if some promising forks are existing */
				if board.maps.attacks_dn[csq] & board.bbs[K | enemy] != 0 {
					score += self.dan_possible[ally][DAN_CHECK];
					if board.maps.attacks_dn[csq] & board.bbs[Q | enemy] != 0 {
						score += self.dan_possible[ally][DAN_ROYAL_FORK];
					}
				} else if board.maps.attacks_dn[csq] & board.bbs[Q | enemy] != 0 && board.maps.attacks_dn[csq] & board.bbs[R | enemy] != 0 {
					score += self.dan_possible[ally][DAN_FORK];
				}
			}
		}

		let bishops = [board.bbs[B], board.bbs[B2]];
		for (ally, mut bb) in bishops.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				let real_atk = board.get_sliding_diagonal_attacks(csq, occup, sides[ally]);
				let imag_atk = board.get_sliding_diagonal_attacks(csq, 0, sides[ally]);
				score += self.pieces_weights[phase][B | ally][csq];
				score += self.mobility_weights[phase][B | ally][real_atk.count_ones() as usize];
				/* Add score if it's an outpost */
				if board.is_outpost(csq, board.bbs[P | ally], board.bbs[P | enemy], ally != 0) {
					score += self.outpost_weights[ally][OUTPOST_BISHOP];
				}
				/* Add score if it's aligned against a king */
				if imag_atk & board.bbs[K | enemy] != 0 && board.get_sliding_diagonal_path_unsafe(csq, kbits[enemy]) & board.bbs[P | ally] == 0 {
					score += self.align_weights[phase][ally][ALIGN_BISHOP];
				} else {
					let asq = imag_atk & board.maps.attacks_king[kbits[enemy]];
					if asq != 0 && board.get_sliding_diagonal_path_unsafe(csq, gtz(asq)) & board.bbs[P | ally] == 0 {
						score += self.align_weights[phase][ally][ALIGN_BISHOP + ALIGN_NEAR];
					}
				}
				/* Battery score will be counted per queen */
			}
		}

		let rooks = [board.bbs[R], board.bbs[R2]];
		for (ally, mut bb) in rooks.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				let real_atk = board.get_sliding_straight_attacks(csq, occup, sides[ally]);
				let imag_atk = board.get_sliding_straight_attacks(csq, 0, sides[ally]);
				score += self.pieces_weights[phase][R | ally][csq];
				score += self.mobility_weights[phase][R | ally][(real_atk).count_ones() as usize];
				/* Add score if it's aligned against a king */
				if imag_atk & board.bbs[K | enemy] != 0 && board.get_sliding_straight_path_unsafe(csq, kbits[enemy]) & board.bbs[P | ally] == 0 {
					score += self.align_weights[phase][ally][ALIGN_ROOK];
				} else {
					let asq = imag_atk & board.maps.attacks_king[kbits[enemy]];
					if asq != 0 && board.get_sliding_straight_path_unsafe(csq, gtz(asq)) & board.bbs[P | ally] == 0 {
						score += self.align_weights[phase][ally][ALIGN_ROOK + ALIGN_NEAR];
					}
				}
				/* Add score if it's in a rook battery */
				if real_atk & board.maps.files[csq] & board.bbs[R | ally] != 0 {
					score += self.battery_weights[ally][BATTERY_RRV];
				} else if real_atk & board.maps.ranks[csq] & board.bbs[R | ally] != 0 {
					score += self.battery_weights[ally][BATTERY_RRH];
				}
			}
		}

		let queens = [board.bbs[Q], board.bbs[Q2]];
		for (ally, mut bb) in queens.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				let rdatk = board.get_sliding_diagonal_attacks(csq, occup, sides[ally]);
				let idatk = board.get_sliding_diagonal_attacks(csq, 0, sides[ally]);
				let rsatk = board.get_sliding_straight_attacks(csq, occup, sides[ally]);
				let isatk = board.get_sliding_straight_attacks(csq, 0, sides[ally]);
				score += self.pieces_weights[phase][Q | ally][csq];
				score += self.mobility_weights[phase][Q | ally][(rdatk | rsatk).count_ones() as usize];
				/* Add score if it's aligned against a king */
				if idatk & board.bbs[K | enemy] != 0 && board.get_sliding_diagonal_path_unsafe(csq, kbits[enemy]) & board.bbs[P | ally] == 0 {
					score += self.align_weights[phase][ally][ALIGN_QUEEN];
				} else if isatk & board.bbs[K | enemy] != 0 && board.get_sliding_straight_path_unsafe(csq, kbits[enemy]) & board.bbs[P | ally] == 0 {
					score += self.align_weights[phase][ally][ALIGN_QUEEN];
				} else {
					let asq = idatk & board.maps.attacks_king[kbits[enemy]];
					if asq != 0 {
						if board.get_sliding_diagonal_path_unsafe(csq, gtz(asq)) & board.bbs[P | ally] == 0 {
							score += self.align_weights[phase][ally][ALIGN_QUEEN + ALIGN_NEAR];
						}
					} else {
						let asq = isatk & board.maps.attacks_king[kbits[enemy]];
						if asq != 0 && board.get_sliding_straight_path_unsafe(csq, gtz(asq)) & board.bbs[P | ally] == 0 {
							score += self.align_weights[phase][ally][ALIGN_QUEEN + ALIGN_NEAR];
						}
					}
				}
				/* Add score if it's in a battery */
				if rsatk & board.maps.files[csq] & board.bbs[R | ally] != 0 {
					score += self.battery_weights[ally][BATTERY_RQV];
				}
				if rdatk & board.bbs[B | ally] != 0 {
					score += self.battery_weights[ally][BATTERY_BQ];
				}
			}
		}

		score += self.mobility_weights[phase][K ][(board.maps.attacks_king[kbits[0]] & !sides[0]).count_ones() as usize];
		score += self.mobility_weights[phase][K2][(board.maps.attacks_king[kbits[1]] & !sides[1]).count_ones() as usize];

		score *= self.turn_weights[(board.turn ^ (score < 0.0)) as usize][TURN_MULT];
		score += self.turn_weights[board.turn as usize][TURN_ADD];
		score += self.rng.gen::<f32>() * self.random_range * 2.0 - self.random_range;

		/* SCORE APPLICATION END */

		if self.cache.len() > CACHE_LIMIT {
			self.cache.clear();
		}
		self.cache.insert(hash, Eval::new(score, 0, board.no));
		Eval::new(score, 0, board.no)
	}
}


#[cfg(test)]
mod tests {
    use super::*;

	#[test]
	fn test_chara_eval_initial() {
		let mut board = Board::default();
		let mut chara = Chara::init(&mut board, 0.0, 1.0);
		let moves = board.get_legal_moves();
		board.make_move(move_transform_back("e2e4", &moves).unwrap());
		let moves = board.get_legal_moves();
		let mov = move_transform_back("e7e5", &moves).unwrap();
		board.make_move(mov);
		let eval = chara.eval(&board);
		assert_eq!((eval.score * 1000.0).round(), (chara.turn_weights[0][TURN_ADD] * 1000.0).round());
		assert_eq!(eval.depth, 2);
		assert_eq!(eval.mate, 0);
	}

	#[test]
	fn test_chara_eval_initial_2() {
		let fens = [
			"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
			"rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
			"rnbqkb1r/pppp1ppp/8/4p2n/4P2N/8/PPPP1PPP/RNBQKB1R w KQkq - 4 4",
			"r3k2r/pbppnpp1/1p1bn2p/4p1q1/4P1Q1/1P1BN2P/PBPPNPP1/R3K2R w KQkq - 2 11"
		];
		for fen in fens.into_iter() {
			let mut board = Board::import(fen);
			let mut chara = Chara::init(&mut board, 0.0, 1.0);
			let eval = chara.eval(&board);
			assert_eq!((eval.score * 1000.0).round(), (chara.turn_weights[0][TURN_ADD] * 1000.0).round());
		}
	}
}