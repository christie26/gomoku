from gomoku import Gomoku


def evaluate_player(state: Gomoku, player: str) -> int:
    """
    Calculates a heuristic score for the given player based on the number and type of patterns
    (e.g., open threes, fours, fives) found in the board.

    Args:
        state: The current game state containing the board.
        player: A string representing the player ("X" or "O").

    Returns:
        An integer score representing how favorable the board is for the given player.
    """

    score = (
        len(state.open_two[player]) * 50
        + len(state.open_three[player]) * 500
        + len(state.open_four[player]) * 1000
        + len(state.five_row[player]) * 10000
    )
    return score


def heuristic_evaluation(state: Gomoku) -> int:
    """
    Evaluates the board state from the perspective of the current player
    by subtracting the opponent's heuristic score from the current player's score.

    Args:
        state: The current game state.

    Returns:
        An integer score indicating which player has the advantage:
            - Positive: Current player is favored
            - Negative: Opponent is favored
            - Zero: Neutral position
    """
    current = evaluate_player(state, state.current_player)
    opponent = evaluate_player(state, "O" if state.current_player == "X" else "X")
    return current - opponent
