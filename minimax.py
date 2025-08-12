from typing import Optional
from faster_functions import (
    Gomoku,
    MoveResult,
    get_candidate_moves,
    heuristic_evaluation,
)
import random
import copy
import pickle

import math
from concurrent.futures import ProcessPoolExecutor

MAX_VALUE = 100000
MIN_VALUE = -100000

MAX_DEPTH = 6


def is_terminal_state(state: Gomoku):
    return state.check_draw() or state.get_winner() is not None


def state_value(state: Gomoku):
    winner = state.get_winner()
    if winner is None:
        return 0
    return MAX_VALUE if winner == "X" else MIN_VALUE


def make_next_state(state: Gomoku, move_x: int, move_y: int) -> Gomoku:
    new_state: Gomoku = state.clone_gomoku()
    new_state.handle_move(move_x, move_y)
    new_state.switch_player()
    return new_state


# def compute_alphabeta_worker(move, state):
#     (move_x, move_y) = move
#     next_state = make_next_state(state, move_x, move_y)
#     value = alphabeta(next_state, alpha, beta, not is_max_player)
#     return (move_x, move_y), value


def get_ai_move(state: Gomoku):
    is_max_player = state.current_player == "X"

    best_value = MIN_VALUE if is_max_player else MAX_VALUE
    alpha, beta = MIN_VALUE, MAX_VALUE

    best_move = None

    # moves = get_candidate_moves(state, 1)

    # with ProcessPoolExecutor(max_workers=30) as executor:
    #     futures = [executor.submit(compute_alphabeta_worker, move, state) for move in moves]
    #     results = [future.result() for future in futures]
    #     best_move, val = max(results, key = lambda x, y: y)
    #     print(f"move: {best_move} score: {val}")
    #     return best_move

    for move_x, move_y in get_candidate_moves(state, 1):
        next_state = make_next_state(state, move_x, move_y)
        value = alphabeta(next_state, alpha, beta, not is_max_player)

        if is_max_player and value > best_value:
            best_value = value
            alpha = max(alpha, best_value)
            best_move = (move_x, move_y)
        elif not is_max_player and value < best_value:
            best_value = value
            beta = min(beta, best_value)
            best_move = (move_x, move_y)

        if alpha >= beta:
            break

    return best_move


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
