import sys

def is_terminal_state(state):
    return True

def state_value(state):
    return 1

def possible_next_states(state):
    pass

def heuristic_evaluation(state):
    return 1

def minimax(state, initiating_player, max_depth = 30):
    if is_terminal_state(state):
        return state_value(state)

    if max_depth == 0:
        return heuristic_evaluation(state)

    minmax_value = None
    for next_state in possible_next_states(state):
        player_win = state.current_player == 'X' ? 1 : -1
        val = minimax(next_state, initiating_player, max_depth - 1)

        if state.current_player != initiating_player and val == player_win:
            return player_win
        elif minimax_value is None or (state.current_player == 'X' and minimax_value < state_value) or (state.current_player == 'O' and minimax_value > state_value) :
            minimax_value = val

    return minimax_value

