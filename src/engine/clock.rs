/* It is entirely possible to add positional evaluation to the equation, e.g. adding/reducing time depending on a positional complexity! */

/// Returns recommended time in ms to spend on searching best move
/// 
/// Has safety padding (10 ms)
pub fn calc_time_to_think(
    turn: bool,
    movetime: Option<u64>,
    wtime: Option<u64>,
    btime: Option<u64>,
    winc: Option<u64>,
    binc: Option<u64>,
    movestogo: Option<u64>
) -> u64 {
    let rem = movetime.unwrap_or({
        if turn {
            btime.unwrap_or(180000)
        } else {
            wtime.unwrap_or(180000)
        }
    });
    let inc = if turn {
        binc.unwrap_or(0)
    } else {
        winc.unwrap_or(0)
    };
    let horizon = if movetime.is_some() {
        1
    } else {
        movestogo.unwrap_or(50)
    };
    let time = (rem / horizon + (inc >> 6) * 61).clamp(10, rem.max(20) - 10);
    time
}