from gomoku import MoveResult
from typing import Optional
from gomoku import Gomoku
import random
import copy
import pickle
from heuristic import heuristic_evaluation
import math

MAX_VALUE = 100000
MIN_VALUE = -100000

MAX_DEPTH = 3


def is_terminal_state(state: Gomoku):
    return state.check_draw() or state.get_winner() is not None


def state_value(state: Gomoku):
    winner = state.get_winner()
    if winner is None:
        return 0
    return MAX_VALUE if winner == "X" else MIN_VALUE


def get_critical_moves(state: Gomoku) -> set[tuple[int, int]]:
    critical_moves = set([])
    for player in [state.opponent_player, state.current_player]:
        for category in [
            "free_three",
            "block_four",
            "open_four",
            "open_three",
            "open_two",
        ]:
            for pattern in getattr(state, category)[player]:
                points_to_check = pattern if category == "free_three" else [pattern[0], pattern[-1]]
                for point in points_to_check:
                    (x, y) = point
                    if (
                        state.board[x][y] == "."
                        and state.is_valid_move(x, y) == MoveResult.VALID
                    ):
                        critical_moves.add((x, y))

    return critical_moves


def get_radius_moves(state: Gomoku, radius: int = 1) -> set[tuple[int, int]]:
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

    return candidates


def get_candidate_moves(state: Gomoku) -> list[tuple[int, int]]:
    candiates = set([])
    candiates.update(get_critical_moves(state))
    candiates.update(get_radius_moves(state))
    return list(candiates)


def make_next_state(state: Gomoku, move_x: int, move_y: int) -> Gomoku:
    new_state: Gomoku = pickle.loads(pickle.dumps(state, -1))
    new_state.handle_move(move_x, move_y)
    new_state.switch_player()
    return new_state


def possible_next_states(state: Gomoku) -> list[Gomoku]:
    return [make_next_state(state, x, y) for x, y in get_candidate_moves(state)]


def get_ai_move(state: Gomoku):
    minimax_value, best_move = None, None
    candidate_moves = get_candidate_moves(state)
    is_max_player = state.current_player == "X"

    for move_x, move_y in candidate_moves:
        # print(f"\n{state.current_player} ({move_x},{move_y})")

        new_state = make_next_state(state, move_x, move_y)
        val = alphabeta(
            new_state, MIN_VALUE, MAX_VALUE, new_state.current_player == "X"
        )
        if (
            minimax_value is None
            or (is_max_player and minimax_value < val)
            or (not is_max_player and minimax_value > val)
        ):
            minimax_value = val
            best_move = (move_x, move_y)

    return best_move


def alphabeta(state: Gomoku, alpha, beta, is_max_player, depth: int = 1) -> int:
    # indent = "  " * (depth + 1)
    if is_terminal_state(state):
        return state_value(state)

    if depth == MAX_DEPTH:
        value = heuristic_evaluation(state)
        # print(f"{state.current_player}{indent}{value}")
        return value

    value = MIN_VALUE if is_max_player else MAX_VALUE

    for next_state in possible_next_states(state):
        if is_max_player:
            value = max(value, alphabeta(next_state, alpha, beta, False, depth + 1))
            alpha = max(alpha, value)
        else:
            value = min(value, alphabeta(next_state, alpha, beta, True, depth + 1))
            beta = min(beta, value)
        # print(f"{state.current_player}{indent}({x},{y})")
        # print(f"{state.current_player}{indent}{alpha} {beta} {value}")
        if alpha >= beta:
            break

    return value


def minimax(state: Gomoku, depth: int = MAX_DEPTH) -> int:
    if is_terminal_state(state):
        return state_value(state)

    if depth == 0:
        return heuristic_evaluation(state)

    minimax_value = None
    for next_state in possible_next_states(state):
        player_win = 1 if state.current_player == "X" else -1
        val = minimax(next_state, depth - 1)

        if val == player_win:
            return player_win
        elif (
            minimax_value is None
            or (state.current_player == "X" and minimax_value < val)
            or (state.current_player == "O" and minimax_value > val)
        ):
            minimax_value = val

    return minimax_value if minimax_value is not None else 0
