from gomoku import MoveResult
from typing import Optional
from gomoku import Gomoku
import random
import copy
import pickle


def is_terminal_state(state: Gomoku):
    return state.check_draw() or state.get_winner() is not None


def state_value(state: Gomoku):
    winner = state.get_winner()
    if winner is None:
        return 0
    return 1 if winner == "X" else -1


def get_candidate_moves(state: Gomoku, radius: int = 1):
    if state.count_empty_spots() == state.size**2:
        return [(random.randint(7, 13), random.randint(7, 13))]
    candidates = set([])
    for row in range(len(state.board)):
        for col in range(len(state.board[0])):
            if state.board[row][col] != ".":
                for dr in range(-radius, radius + 1):
                    for dc in range(-radius, radius + 1):
                        new_row, new_col = row + dr, col + dc
                        if state.is_valid_move(new_row, new_col) == MoveResult.VALID:
                            candidates.add((new_row, new_col))
    return list(candidates)


def possible_next_states(state: Gomoku):
    states = []
    for move_x, move_y in get_candidate_moves(state):
        new = pickle.loads(pickle.dumps(state, -1))
        # new = copy.deepcopy(state)
        result, _ = new.handle_move(move_x, move_y)
        new.switch_player()
        if result != MoveResult.VALID:
            print("fatal", result, move_x, move_y)
            raise Exception("AI tried to do an invalid move, should be impossible")
        states.append(new)
    return states


def get_ai_move(state: Gomoku):
    max_val, x, y = None, None, None
    for move_x, move_y in get_candidate_moves(state):
        # new = copy.deepcopy(state)
        new = pickle.loads(pickle.dumps(state, -1))
        result, _ = new.handle_move(move_x, move_y)
        new.switch_player()
        val = minimax(new, state.current_player)
        if max_val is None or val < max_val:
            max_val, x, y = val, move_x, move_y
    return x, y


def heuristic_evaluation(state: Gomoku):

    return 0


def minimax(state: Gomoku, initiating_player: str, max_depth: int = 3) -> int:
    if is_terminal_state(state):
        return state_value(state)

    if max_depth == 0:
        return heuristic_evaluation(state)

    minimax_value: Optional(int) = None
    for next_state in possible_next_states(state):
        # print("depth", max_depth, "\n\n")
        # next_state.print_board()
        player_win = 1 if state.current_player == "X" else -1
        val = minimax(next_state, initiating_player, max_depth - 1)

        if val == player_win:
            return player_win
        elif (
            minimax_value is None
            or (state.current_player == "X" and minimax_value < val)
            or (state.current_player == "O" and minimax_value > val)
        ):
            minimax_value = val

    return minimax_value if minimax_value is not None else 0
