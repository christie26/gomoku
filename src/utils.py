def load_and_validate_board(filepath):
    with open(filepath, "r") as f:
        lines = [line.strip() for line in f if line.strip()]

    if len(lines) != 19 or any(len(row) != 19 for row in lines):
        raise ValueError("Board must be 19x19")

    valid_symbols = {".", "X", "O"}
    board = []
    count = {"X": 0, "O": 0}

    for row in lines:
        if any(c not in valid_symbols for c in row):
            raise ValueError("Board can only contain '.', 'X', or 'O'")
        for c in row:
            if c in count:
                count[c] += 1
        board.append(list(row))

    current_player = "X"

    return board, current_player


def load_board_str(filepath):
    with open(filepath, "r") as f:
        content = f.read()
        return content


def load_history(filepath):
    with open(filepath, "r") as f:
        content = f.read()
        historys = content.removeprefix("move history:").strip()
        history_array = historys.split("->")
        history_tuples = [
            (int(array.strip("()").split(",")[0]), int(array.strip("()").split(",")[1]))
            for array in history_array
        ]

        return history_tuples
