from gomoku import Gomoku


def extract_all_lines(state: Gomoku) -> list[str]:
    """
    Extracts all possible lines (rows, columns, diagonals) from the board as strings.

    Args:
        state: The current game state, which includes a 2D board represented as a list of strings.

    Returns:
        A list of strings, each representing a line on the board (row, column, or diagonal).
    """
    size = len(state.board)
    lines = []

    # Rows and columns
    for i in range(size):
        row = "".join(state.board[i])
        col = "".join(state.board[j][i] for j in range(size))
        lines.append(row)
        lines.append(col)

    # Diagonals (↘ direction)
    for d in range(-size + 1, size):
        diag1 = "".join(
            state.board[i][i - d] for i in range(max(d, 0), min(size, size + d))
        )
        lines.append(diag1)

    # Diagonals (↙ direction)
    for d in range(2 * size - 1):
        diag2 = []
        for i in range(size):
            j = d - i
            if 0 <= i < size and 0 <= j < size:
                diag2.append(state.board[i][j])
        lines.append("".join(diag2))

    return lines


def evaluate_player(state, player: str) -> int:
    """
    Calculates a heuristic score for the given player based on the number and type of patterns
    (e.g., open threes, fours, fives) found in the board.

    Args:
        state: The current game state containing the board.
        player: A string representing the player ("X" or "O").

    Returns:
        An integer score representing how favorable the board is for the given player.
    """
    lines = extract_all_lines(state)

    score = 0
    patterns = {
        f"{player*5}": 100000,
        f".{player*4}.": 10000,
        f"{player*4}.": 1000,
        f".{player*4}": 1000,
        f".{player*3}.": 500,
        f".{player*2}.": 50,
    }

    for line in lines:
        for pattern, value in patterns.items():
            score += line.count(pattern) * value

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
