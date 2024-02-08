pub struct Weights {
    /* TODO: add comments (this comment is real) */
    pub pieces_weights_square_related:  [[[f32; 64]; 6]; 2],
    pub pieces_weights_const:            [[f32;  6]; 2],
    pub mobility_weights_const:          [[f32; 32]; 6],
    pub turn_weights_pre:                 [f32;  2],
    pub align_weights_pre:               [[f32;  6]; 2],
    pub battery_weights_pre:              [f32;  4],
    pub dp_weight:                         f32,
    pub pp_weights:                       [f32;  3],
    pub outpost_weight_pre:               [f32;  2],
    pub dan_possible_pre:                 [f32;  3]
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
			[ 0.91,  3.0 ,  3.1 ,  5.0 ,  9.0 ,  0.0 ]
		];

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

		let mut turn_weights_pre	=  [ 1.15,  0.1 ];
		let mut align_weights_pre   = [[ 0.35,  0.3 ,  0.3 ,  0.21,  0.15,  0.3 ], [ 0.2 ,  0.2 ,  0.1 ,  0.05,  0.1,  0.1 ]];
		let mut battery_weights_pre =  [ 0.21,  0.22,  0.45,  0.11];
		let mut dp_weight			=    0.75;
		let mut pp_weights			=  [ 1.21,  1.11,  1.42];
		let mut outpost_weight_pre  =  [ 0.4 ,  0.2 ];
		let mut dan_possible_pre    =  [ 0.4 ,  0.6 ,  0.3 ];

        Self {
            pieces_weights_square_related,
            pieces_weights_const,
            mobility_weights_const,
            turn_weights_pre,
            align_weights_pre,
            battery_weights_pre,
            dp_weight,
            pp_weights,
            outpost_weight_pre,
            dan_possible_pre
        }
    }
}