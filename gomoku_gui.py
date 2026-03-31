import tkinter as tk
from lib_gomoku import Gomoku, MoveResult, get_ai_move
import argparse
import time
import threading
from setting_panel import SettingsPanel
from player_panel import Player, PlayerPanel
from board_canvas import BoardCanvas

CELL_SIZE = 32
LABEL_PADDING = 10
BOARD_SIZE = 19
PADDING = 30

LIGHT_BACKGROUND = "#FAEBD7"
SELECT_BACKGROUND = "#A68A64"
BORDER_COLOR = "#6f5c43"

LABEL_FONT = "Phosphate"
NAME_FONT = "Rockwell"


class GomokuGUI:
    def __init__(self, root, player1, player2, history=None):
        self.root = root
        self.root.title("Gomoku")

        self.sizeeee = CELL_SIZE * (BOARD_SIZE - 1) + PADDING * 2
        self.game = Gomoku(size=BOARD_SIZE)
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
        self.right_frame.pack(side="right", fill="y")
        self.right_frame.pack_propagate(False)

        # ===== CANVAS =====
        self.canvas = BoardCanvas(
            self.left_frame, self.sizeeee, self.game, self.handle_click
        )

        # ===== UNDO/REDO =====
        self.state_history = [self.game.clone_gomoku()]
        self.history_index = 0

        self.player1_name = player1
        self.player2_name = player2
        self.players = {
            "X": Player(True, player1, True),
            "O": Player(False, player2, True),
        }

        # ===== PLAYER BOXES =====
        self.player_frames = {
            "X": PlayerPanel(self.root, self.right_frame, self.players["X"]),
            "O": PlayerPanel(self.root, self.right_frame, self.players["O"]),
        }

        # ===== SETTINGS =====
        self.setting_panel = SettingsPanel(
            self.right_frame,
            on_start_game=self.start_game,
            on_undo=self.undo,
            on_redo=self.redo,
            on_debug=self.debug_onoff,
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
        if self.players[self.game.current_player].is_human and self.is_playing:
            x = round((event.x - PADDING) / CELL_SIZE)
            y = round((event.y - PADDING) / CELL_SIZE)
            self.play_one_turn(y, x)
        else:
            return

    # ===== GAME FLOW =====
    def play_one_turn(self, x, y):
        result = self.game.is_valid_move(x, y)

        if result == MoveResult.VALID:
            _, capture = self.game.handle_move(x, y)
            if capture:
                self.update_captures(self.game.current_player)

            self.canvas.draw_stones(self.game.board)

            winner = self.game.get_winner()
            self.canvas.draw_last_move(y, x)
            if winner:
                self.finish_game(winner)
            else:
                self.change_turn()
                if not self.players[self.game.current_player].is_human:
                    self.ai_play()

            # Record state for undo/redo
            self.state_history = self.state_history[: self.history_index + 1]
            self.state_history.append(self.game.clone_gomoku())
            self.history_index += 1
            self.update_undo_redo_buttons()

    def start_game(self):
        self.game = Gomoku(size=BOARD_SIZE)
        self.canvas.set_game(self.game)

        ruleset = self.setting_panel.ruleset.get()
        print(f"Game is started with {ruleset} ruleset")

        p = self.game.current_player
        self.highlight_active_player()
        self.start_turn_timer(p)

        self.is_playing = True
        self.canvas.reset_board(self.is_playing)
        self.setting_panel.reset_panel(self.is_playing)
        self.player_frames["X"].reset_panel()
        self.player_frames["O"].reset_panel()

    def change_turn(self):
        self.end_turn_timer(self.game.current_player)

        self.game.switch_player()
        p = self.game.current_player

        self.highlight_active_player()
        self.start_turn_timer(p)

    def ai_play(self):
        def run_ai():
            mv, moves = get_ai_move(self.game)

            if mv:
                self.canvas.delete("debug")
                x, y, _ = mv
                for m in moves:
                    x1, y1, score1 = m
                    selected = x == x1 and y == y1
                    if self.debug:
                        self.draw_possible_stone(y1, x1, "O", score1, selected)
                self.root.after(0, lambda: self.play_one_turn(x, y))

        threading.Thread(target=run_ai, daemon=True).start()

    def finish_game(self, winner):
        self.end_turn_timer(self.game.current_player)
        self.canvas.show_winner(f"{self.players[winner].name} wins")
        self.is_playing = False
        self.setting_panel.reset_panel(False)

    # ===== PLAYER PANEL =====
    def update_captures(self, player, count=1):
        self.player_frames[player].add_capture()

    def highlight_active_player(self):
        for p in ["X", "O"]:
            if p == self.game.current_player:
                self.player_frames[p].hightlight_player()
            else:
                self.player_frames[p].unhightlight_player()

    def start_turn_timer(self, p: Player):
        self.player_frames[p].start_timer()

    def end_turn_timer(self, p: Player):
        self.player_frames[p].stop_timer()

    # ===== UNDO/REDO =====
    def undo(self, event=None):
        self.canvas.remove_last_move()
        if self.history_index <= 0:
            return
        self.history_index -= 1
        self.game = self.state_history[self.history_index].clone_gomoku()
        self.canvas.draw_stones(self.game.board)
        current_move = self.game.current_move
        if current_move:
            self.canvas.draw_last_move(current_move[1], current_move[0])
        self.update_undo_redo_buttons()
        self.end_turn_timer("X")
        self.end_turn_timer("O")

    def redo(self, event=None):
        if self.history_index >= len(self.state_history) - 1:
            return
        self.history_index += 1
        self.game = self.state_history[self.history_index].clone_gomoku()
        self.canvas.draw_stones(self.game.board)
        current_move = self.game.current_move
        if current_move:
            self.canvas.remove_last_move()
            self.canvas.draw_last_move(current_move[1], current_move[0])
        self.update_undo_redo_buttons()

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
        if not debug:
            self.canvas.delete("debug")

    def switch_play_mode(self, play_mode):
        if play_mode == "pvp":
            self.player_frames["X"].update_player_type(True)
            self.player_frames["O"].update_player_type(True)
        elif play_mode == "pvsa":
            self.player_frames["X"].update_player_type(True)
            self.player_frames["O"].update_player_type(False)
        elif play_mode == "avsp":
            self.player_frames["X"].update_player_type(False)
            self.player_frames["O"].update_player_type(True)


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
