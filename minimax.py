from typing import Optional
from faster_functions import Gomoku, MoveResult, get_candidate_moves
import random
import copy
import pickle
from heuristic import heuristic_evaluation
import math
from concurrent.futures import ProcessPoolExecutor

MAX_VALUE = 100000
MIN_VALUE = -100000

MAX_DEPTH = 5


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
                points_to_check = (
                    pattern if category == "free_three" else [pattern[0], pattern[-1]]
                )
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


# NOTE same name function in Rust
# def get_candidate_moves(state: Gomoku) -> list[tuple[int, int]]:
#     candiates = set([])
#     candiates.update(get_critical_moves(state))
#     candiates.update(get_radius_moves(state))
#     return list(candiates)


def make_next_state(state: Gomoku, move_x: int, move_y: int) -> Gomoku:
    new_state: Gomoku = state.clone_gomoku()
    new_state.handle_move(move_x, move_y)
    new_state.switch_player()
    return new_state


def  compute_alphabeta_worker(move, state):
    (move_x, move_y) = move
    next_state = make_next_state(state, move_x, move_y)
    value = alphabeta(next_state, alpha, beta, not is_max_player)
    return (move_x, move_y), value

def get_ai_move(state: Gomoku):
    is_max_player = state.current_player == "X"

    best_value = MIN_VALUE if is_max_player else MAX_VALUE
    alpha, beta = MIN_VALUE, MAX_VALUE

    best_move = None

    moves = get_candidate_moves(state, 1)

    with ProcessPoolExecutor(max_workers=30) as executor:
        futures = [executor.submit(compute_alphabeta_worker, move, state) for move in moves]
        results = [future.result() for future in futures]
        best_move, val = max(results, key = lambda x, y: y)
        print(f"move: {best_move} score: {val}")
        return best_move

    # for move_x, move_y in get_candidate_moves(state, 1):
    #     next_state = make_next_state(state, move_x, move_y)
    #     value = alphabeta(next_state, alpha, beta, not is_max_player)
    #
    #     if is_max_player and value > best_value:
    #         best_value = value
    #         alpha = max(alpha, best_value)
    #         best_move = (move_x, move_y)
    #     elif not is_max_player and value < best_value:
    #         best_value = value
    #         beta = min(beta, best_value)
    #         best_move = (move_x, move_y)
    #
    #     if alpha >= beta:
    #         break
    #
    # return best_move


def alphabeta(state: Gomoku, alpha, beta, is_max_player, depth: int = 1) -> int:
    if is_terminal_state(state):
        return state_value(state)

    if depth == MAX_DEPTH:
        value = heuristic_evaluation(state)
        return value

    value = MIN_VALUE if is_max_player else MAX_VALUE

    for move_x, move_y in get_candidate_moves(state, 1):
        next_state = make_next_state(state, move_x, move_y)

        if is_max_player:
            value = max(value, alphabeta(next_state, alpha, beta, False, depth + 1))
            alpha = max(alpha, value)
        else:
            value = min(value, alphabeta(next_state, alpha, beta, True, depth + 1))
            beta = min(beta, value)
        if alpha >= beta:
            break

    return value
