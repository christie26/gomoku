from gomoku import MoveResult
from typing import Optional
from gomoku import Gomoku
import random
import copy


def is_terminal_state(state: Gomoku):
    return state.check_winner() or state.check_draw()


def state_value(state: Gomoku):
    if state.check_winner():
        return 1 if state.current_player == "X" else -1
    return 0


def get_candidate_moves(state: Gomoku, radius=2):
    if state.count_empty_spots() == state.size**2:
        return [(random.randint(7, 13), random.randint(7, 13))]
    candidates = set([])
    for row in range(len(state.board)):
        for col in range(len(state.board[0])):
            if state.board[row][col] != ".":
                for dr in range(-radius, radius + 1):
                    for dc in range(-radius, radius + 1):
                        new_row, new_col = row + dr, col + dc
                        if (
                            state.is_valid_move(new_row, new_col)
                            and state.board[new_row][new_col] == "."
                        ):
                            candidates.add((new_row, new_col))
    return list(candidates)


def possible_next_states(state: Gomoku):
    states = []
    for move_x, move_y in get_candidate_moves(state):
        new = copy.deepcopy(state)
        result, _ = new.handle_move(move_x, move_y)
        if result != MoveResult.VALID:
            raise Exception("AI tried to do an invalid move, should be impossible")
        states.append(new)
    return states


def heuristic_evaluation(state: Gomoku):

    return 0


def minimax(state: Gomoku, initiating_player: str, max_depth: int = 30) -> int:
    if is_terminal_state(state):
        return state_value(state)

    if max_depth == 0:
        return heuristic_evaluation(state)

    minimax_value: Optional(int) = None
    for next_state in possible_next_states(state):
        player_win = 1 if state.current_player == "X" else -1
        val = minimax(next_state, initiating_player, max_depth - 1)

        if state.current_player != initiating_player and val == player_win:
            return player_win
        elif (
            minimax_value is None
            or (state.current_player == "X" and minimax_value < state_value)
            or (state.current_player == "O" and minimax_value > state_value)
        ):
            minimax_value = val

    return minimax_value if minimax is not None else 0
