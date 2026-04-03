import tkinter as tk
from tkinter import simpledialog
from lib_gomoku import Gomoku, MoveResult, get_ai_move, get_move_pv, get_hint
import argparse
import time
import threading
from setting_panel import SettingsPanel
from player_panel import Player, PlayerPanel
from board_canvas import BoardCanvas
from constants import (
    CELL_SIZE,
    BOARD_SIZE,
    PADDING,
    LIGHT_BACKGROUND,
    BORDER_COLOR,
)


class GomokuGUI:
    def __init__(self, root, player1_name, player2_name, history=None):
        self.root = root
        self.root.title("Gomoku")

        self.sizeeee = CELL_SIZE * (BOARD_SIZE - 1) + PADDING * 2
        self.is_playing = False
        self.debug = True

        # ===== MAIN LAYOUT =====
        self.main_frame = tk.Frame(
            root,
            highlightbackground=BORDER_COLOR,
            highlightthickness=2,
        )
        self.main_frame.pack()

        self.left_frame = tk.Frame(self.main_frame)
        self.left_frame.pack(side="left")

        self.right_frame = tk.Frame(
            self.main_frame,
            width=250,
            background=LIGHT_BACKGROUND,
            highlightbackground=BORDER_COLOR,
            highlightthickness=2,
        )
        self.canvas = BoardCanvas(self.left_frame, self.sizeeee, self.handle_click)

        self.right_frame.pack(side="right", fill="y")
        self.right_frame.pack_propagate(False)
        self.right_frame.bind("<Enter>", self.canvas.remove_hover)

        # ===== UNDO/REDO =====
        self.game = Gomoku(size=BOARD_SIZE)
        self.state_history = [self.game.clone_gomoku()]
        self.history_index = 0

        self.players = {
            "X": Player(
                True,
                player1_name,
                True,
                PlayerPanel(
                    self.root,
                    self.right_frame,
                    player1_name,
                    True,
                    True,
                ),
            ),
            "O": Player(
                False,
                player2_name,
                True,
                PlayerPanel(
                    self.root,
                    self.right_frame,
                    player2_name,
                    False,
                    True,
                ),
            ),
        }

        # ===== PLAYER BOXES =====

        # ===== SETTINGS =====
        self.setting_panel = SettingsPanel(
            self.right_frame,
            on_start_game=self.start_game,
            on_undo=self.undo,
            on_redo=self.redo,
            on_debug=self.debug_onoff,
            on_hint=self.show_hint,
            on_play_mode=self.switch_play_mode,
        )

        # ===== HISTORY =====
        if history:
            for x, y in history:

                self.play_one_turn(x, y)

        self.root.bind("<Left>", self.undo)
        self.root.bind("<Right>", self.redo)

    # ===== HANDLE INPUT ====
    def handle_click(self, event):
        if self.is_playing:
            x = round((event.x - PADDING) / CELL_SIZE)
            y = round((event.y - PADDING) / CELL_SIZE)
            # Ctrl+click: print the PV for this position instead of playing
            if event.state & 0x4:  # Ctrl key modifier
                row, col = y, x  # board coords: row=y, col=x
                if 0 <= row < BOARD_SIZE and 0 <= col < BOARD_SIZE:
                    print(f"\n--- PV analysis for {chr(ord('A') + col)}{row + 1} ---")
                    pv, score = get_move_pv(self.game, row, col)
                    pv_str = " -> ".join(f"{chr(ord('A') + c)}{r + 1}" for r, c in pv)
                    print(f"  Score: {score}")
                    print(f"  PV: {pv_str}")
                    print(f"  Depth: {len(pv)} moves")
                return
            self.play_one_turn(y, x)  # note: board is row (y), col (x)

    # ===== GAME FLOW =====

    def play_one_turn(self, x, y):
        self.canvas.remove_debug()
        self.canvas.remove_hint()

        result = self.game.is_valid_move(x, y)

        if result == MoveResult.VALID:
            result, capture_count, captured = self.game.handle_move(x, y)
            if captured:
                self.canvas.show_capture(captured)
            self.players[self.game.current_player].panel.update_capture(
                self.game.capture_count[self.game.current_player]
            )

            self.canvas.draw_stones(self.game.board)

            winner = self.game.get_winner()
            self.canvas.draw_last_move(y, x)
            if winner:
                self.finish_game(winner)
            else:
                self.change_turn()
                p = self.game.current_player
                if self.debug:
                    self.show_debug()
                if not self.players[p].is_human:
                    self.ai_play()

            # Record state for undo/redo
            self.state_history = self.state_history[: self.history_index + 1]
            self.state_history.append(self.game.clone_gomoku())
            self.history_index += 1
            self.update_undo_redo_buttons()

    def start_game(self):
        self.set_game(Gomoku(size=BOARD_SIZE))
        self.canvas.set_game(self.game)

        ruleset = self.setting_panel.ruleset.get()
        print(f"Game is started with {ruleset} ruleset")

        p = self.game.current_player
        self.highlight_active_player()
        self.start_turn_timer(p)

        self.is_playing = True
        self.canvas.reset_board(self.is_playing)
        self.setting_panel.reset_panel(self.is_playing)
        self.players["X"].panel.reset_panel()
        self.players["O"].panel.reset_panel()

    def change_turn(self):
        self.end_turn_timer(self.game.current_player)

        self.game.switch_player()
        p = self.game.current_player

        self.highlight_active_player()
        self.start_turn_timer(p)

    def ai_play(self):
        def run_ai():
            best_move, moves = get_ai_move(self.game)
            x, y, _ = best_move
            if self.debug:
                self.canvas.draw_debug(moves, best_move)
                # QUESTION - what happens if there's no best_move ?
            self.root.after(0, lambda: self.play_one_turn(x, y))

        threading.Thread(target=run_ai, daemon=True).start()

    def finish_game(self, winner):
        self.end_turn_timer(self.game.current_player)
        self.canvas.show_winner(f"{self.players[winner].name} wins")
        self.is_playing = False
        self.setting_panel.reset_panel(False)

    # ===== DEBUG/HINT =====
    def show_hint(self):
        # TODO add variable so that it's not triggered twice
        best_move, _ = get_ai_move(self.game)
        self.canvas.draw_hint(best_move)

    def show_debug(self):
        best_move, moves = get_ai_move(self.game)
        self.canvas.draw_debug(moves, best_move)

    # ===== PLAYER PANEL =====
    def highlight_active_player(self):
        for p in ["X", "O"]:
            if p == self.game.current_player:
                self.players[p].panel.hightlight_player()
            else:
                self.players[p].panel.unhightlight_player()

    def start_turn_timer(self, p: Player):
        self.players[p].panel.start_timer()

    def end_turn_timer(self, p: Player):
        self.players[p].panel.stop_timer()

    # ===== UNDO/REDO =====
    def undo(self, event=None):
        self.canvas.remove_last_move()
        if self.history_index <= 0:
            return
        self.history_index -= 1
        self.set_game(self.state_history[self.history_index].clone_gomoku())

        self.canvas.draw_stones(self.game.board)
        current_move = self.game.current_move
        if current_move:
            self.canvas.draw_last_move(current_move[1], current_move[0])

        for p in ["X", "O"]:
            self.players[p].panel.update_capture(self.game.capture_count[p])

        self.update_undo_redo_buttons()

        self.highlight_active_player()
        self.end_turn_timer("X")
        self.end_turn_timer("O")

    def redo(self, event=None):
        if self.history_index >= len(self.state_history) - 1:
            return
        self.history_index += 1
        self.set_game(self.state_history[self.history_index].clone_gomoku())
        self.canvas.draw_stones(self.game.board)
        current_move = self.game.current_move
        if current_move:
            self.canvas.remove_last_move()
            self.canvas.draw_last_move(current_move[1], current_move[0])
        for p in ["X", "O"]:
            self.players[p].panel.update_capture(self.game.capture_count[p])

        self.update_undo_redo_buttons()

        self.highlight_active_player()

    def update_undo_redo_buttons(self):
        self.setting_panel.undo_button.config(
            state=tk.NORMAL if self.history_index > 0 else tk.DISABLED
        )
        self.setting_panel.redo_button.config(
            state=(
                tk.NORMAL
                if self.history_index < len(self.state_history) - 1
                else tk.DISABLED
            )
        )

    # ===== SETTING =====
    def debug_onoff(self, debug: bool):
        self.debug = debug
        if debug:
            self.show_debug()
        else:
            self.canvas.remove_debug()

    def switch_play_mode(self, play_mode):
        if play_mode == "pvp":
            self.players["X"].panel.update_player_type(True)
            self.players["O"].panel.update_player_type(True)
            self.players["X"].panel.set_hint_available(True)
            self.players["O"].panel.set_hint_available(True)
        elif play_mode == "pvsa":
            self.players["X"].panel.update_player_type(True)
            self.players["O"].panel.update_player_type(False)
            self.players["X"].panel.set_hint_available(False)
            self.players["O"].panel.set_hint_available(False)
        elif play_mode == "avsp":
            self.players["X"].panel.update_player_type(False)
            self.players["O"].panel.update_player_type(True)
            self.players["X"].panel.set_hint_available(False)
            self.players["O"].panel.set_hint_available(False)

    def set_game(self, game):
        self.game = game
        self.canvas.set_game(game)


def load_board_str(filepath):
    with open(filepath, "r") as f:
        content = f.read()
        return content


def chess_to_xy(coord: str):
    """Convert chess coordinate (e.g. 'a1', 's19') to (y, x) 0-indexed."""
    col = ord(coord[0].lower()) - ord("a")
    row = int(coord[1:]) - 1
    if not (0 <= col < 19 and 0 <= row < 19):
        raise ValueError(f"Coordinate {coord} out of bounds for 19x19 board")
    return row, col


def load_history(filepath):
    with open(filepath, "r") as f:
        content = f.read()
        lines = content.splitlines()
        for line in lines:
            if line.strip() != "":
                content = line
                break
        historys = content.removeprefix("move history:").strip()
        history_array = historys.split("->")
        history_tuples = [
            # (int(array.strip("()").split(",")[0]), int(array.strip("()").split(",")[1]))
            chess_to_xy(array)
            for array in history_array
        ]

        return history_tuples


# ===== MAIN =====
if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--black", type=str, default=None)
    parser.add_argument("--white", type=str, default=None)

    parser.add_argument("--board", type=str, help="Path to board file")

    parser.add_argument("--history", type=str, help="Path to move history file")

    parser.add_argument(
        "--history-until", type=int, help="index of history where you want to stop"
    )

    args = parser.parse_args()
    board = None
    current_player = "X"  # default

    if args.board:
        try:
            # board, current_player = load_and_validate_board(args.board)
            board = load_board_str(args.board)
            print(f"Loaded board from {args.board}")
        except Exception as e:
            print(f"Failed to load board: {e}")
            exit(1)

    history = None

    if args.history:
        try:
            history = load_history(args.history)
            print(history, len(history))
            if args.history_until:
                history = history[: args.history_until]
                print(args.history_until)
                print(history, len(history))
        except Exception as e:
            print(f"Failed to load history: {e}")
            exit(1)
    root = tk.Tk()
    app = GomokuGUI(root, args.black, args.white, history)

    # if board is passed, update game state
    if board:
        app.game.parse_board(board)
        # app.game.board = board
        # app.game.current_player = current_player
        # app.game.opponent_player = "O" if current_player == "X" else "X"
        app.draw_board(app.game.board)
        # Reset history to match the loaded board
        app.state_history = [app.game.clone_gomoku()]
        app.history_index = 0
        app.update_undo_redo_buttons()

    root.mainloop()
