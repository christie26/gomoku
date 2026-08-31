import tkinter as tk
from lib_gomoku import Gomoku, MoveResult, get_ai_move_stats, get_minimax_eval
import argparse
import time
import threading
from src.setting_panel import SettingsPanel
from src.player_panel import Player, PlayerPanel
from src.board_canvas import BoardCanvas
from src.score_bar import ScoreBar
from src.screen_constant import CELL_SIZE, LABEL_PADDING, BOARD_SIZE, PADDING, LIGHT_BACKGROUND, SELECT_BACKGROUND, BORDER_COLOR, LABEL_FONT, NAME_FONT, SETTING_PANEL_WIDTH


class GomokuGUI:
    def __init__(self, root, player1, player2, history=None, use_undo=False):
        self.root = root
        self.root.title("Gomoku")

        self.sizeeee = CELL_SIZE * (BOARD_SIZE - 1) + PADDING * 2
        self.is_playing = False
        self.debug = False
        self.ai_thinking = False
        self.ai_stats = []  # list of (elapsed_secs, nodes_visited, pruning_percent)
        self.use_undo = use_undo

        # ===== MAIN LAYOUT =====
        self.main_frame = tk.Frame(
            root,
            highlightbackground=BORDER_COLOR,
            highlightthickness=2,
        )
        self.main_frame.pack()

        self.score_bar = ScoreBar(self.main_frame, self.sizeeee)

        self.left_frame = tk.Frame(self.main_frame)
        self.left_frame.pack(side="left")

        self.right_frame = tk.Frame(
            self.main_frame,
            width=SETTING_PANEL_WIDTH,
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

        self.player1_name = player1
        self.player2_name = player2
        self.players = {
            "X": Player(True, player1, True),
            "O": Player(False, player2, True),
        }
        self.canvas.set_players(self.players)

        # ===== PLAYER BOXES =====
        self.player_frames = {
            "X": PlayerPanel(self.root, self.right_frame, self.players["X"]),
            "O": PlayerPanel(self.root, self.right_frame, self.players["O"]),
        }

        # ===== SETTINGS =====
        self.setting_panel = SettingsPanel(
            self.right_frame,
            on_start_game=self.start_game,
            on_end_game=self.end_game,
            on_undo=self.undo,
            on_redo=self.redo,
            on_debug=self.debug_onoff,
            on_play_mode=self.switch_play_mode,
            on_hint=self.show_hint,
            default_debug = self.debug,
            use_undo=self.use_undo
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
    def play_one_turn(self, x, y, score = None):
        if self.players[self.game.current_player].is_human or not self.debug:
            self.canvas.delete_debug()
        result = self.game.is_valid_move(x, y)

        if result == MoveResult.VALID:
            result, capture_count, captured = self.game.handle_move(x, y)
            if score is None:
              score = get_minimax_eval(self.game)
            self.setting_panel.update_score(score)
            self.score_bar.update_score(score)
            if captured:
                self.canvas.show_capture(captured)
            self.player_frames[self.game.current_player].update_capture(
                self.game.capture_count[self.game.current_player]
            )

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
            self.game.print_state()

    def start_game(self):
        self.set_game(Gomoku(size=BOARD_SIZE))
        self.state_history = [self.game.clone_gomoku()]
        self.history_index = 0

        # ruleset = self.setting_panel.ruleset.get()
        # print(f"Game is started with {ruleset} ruleset")

        p = self.game.current_player
        self.highlight_active_player()
        self.start_turn_timer(p)

        self.is_playing = True
        self.canvas.reset_board(self.is_playing)
        self.ai_stats = []
        self.setting_panel.reset_panel(self.is_playing)
        self.setting_panel.reset_ai_stats()
        self.score_bar.update_score(0)
        self.player_frames["X"].reset_panel()
        self.player_frames["O"].reset_panel()
        if not self.players[self.game.current_player].is_human:
            self.ai_play()

    def end_game(self):
        p = self.game.current_player
        self.end_turn_timer(p)
        self.player_frames[p].unhightlight_player()
        
        self.set_game(Gomoku(size=BOARD_SIZE))
        self.canvas.set_game(self.game)

        self.is_playing = False
        self.canvas.reset_board(self.is_playing)
        self.ai_stats = []
        self.setting_panel.reset_panel(self.is_playing)
        self.setting_panel.reset_ai_stats()
        self.player_frames["X"].reset_panel()
        self.player_frames["O"].reset_panel()

    def change_turn(self):
        self.end_turn_timer(self.game.current_player)

        self.game.switch_player()
        p = self.game.current_player

        self.highlight_active_player()
        self.start_turn_timer(p)
        if not self.players[p].is_human:
            self.canvas.remove_hover()

    def ai_play(self):
        self.ai_thinking = True
        self.update_undo_redo_buttons()

        def run_ai():
            mv, moves, elapsed, nodes, pruning = get_ai_move_stats(self.game)

            def apply_move():
                self.ai_thinking = False
                if mv:
                    if self.debug:
                        self.canvas.delete_debug()
                    x, y, score = mv
                    for m in moves:
                        x1, y1, score1 = m
                        selected = x == x1 and y == y1
                        if self.debug:
                            self.canvas.draw_possible_stone(y1, x1, "O", score1, selected)
                    self.ai_stats.append((elapsed, nodes, pruning))
                    self.setting_panel.update_ai_stats(self.ai_stats)
                    self.play_one_turn(x, y, score)
                self.update_undo_redo_buttons()

            self.root.after(0, apply_move)

        threading.Thread(target=run_ai, daemon=True).start()

    def finish_game(self, winner):
        self.end_turn_timer(self.game.current_player)
        winner_text = self.players[winner].name if self.players[winner].is_human else "AI"
        self.canvas.show_winner(f"{winner_text} wins")
        self.canvas.finish_board()
        
        self.is_playing = False
        self.setting_panel.reset_panel(self.is_playing)

    def show_hint(self):
        mv, _, _, _, _ = get_ai_move_stats(self.game)
        x, y, _ = mv
        self.canvas.draw_possible_stone(y, x, self.game.current_player, None, True)

    # ===== PLAYER PANEL =====
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
        if self.ai_thinking:
            return
        self.canvas.remove_last_move()
        self.canvas.delete_debug()
        if self.history_index <= 0:
            return
        self.history_index -= 1
        # Skip over AI moves so undo always lands back on a human turn.
        while self.history_index > 0 and not self.players[
            self.state_history[self.history_index].current_player
        ].is_human:
            self.history_index -= 1
        self.set_game(self.state_history[self.history_index].clone_gomoku())

        self.canvas.draw_stones(self.game.board)
        current_move = self.game.current_move
        if current_move:
            self.canvas.draw_last_move(current_move[1], current_move[0])

        for p in ["X", "O"]:
            self.player_frames[p].update_capture(self.game.capture_count[p])

        self.update_undo_redo_buttons()

        self.highlight_active_player()
        self.end_turn_timer("X")
        self.end_turn_timer("O")

    def redo(self, event=None):
        if self.ai_thinking:
            return
        if self.history_index >= len(self.state_history) - 1:
            return
        self.history_index += 1
        # Skip over AI moves so redo always lands back on a human turn (or the end).
        while (
            self.history_index < len(self.state_history) - 1
            and not self.players[
                self.state_history[self.history_index].current_player
            ].is_human
        ):
            self.history_index += 1
        self.set_game(self.state_history[self.history_index].clone_gomoku())
        self.canvas.delete_debug()
        self.canvas.draw_stones(self.game.board)
        current_move = self.game.current_move
        if current_move:
            self.canvas.remove_last_move()
            self.canvas.draw_last_move(current_move[1], current_move[0])
        for p in ["X", "O"]:
            self.player_frames[p].update_capture(self.game.capture_count[p])

        self.update_undo_redo_buttons()

        self.highlight_active_player()

    def update_undo_redo_buttons(self):
        if self.use_undo:
          if self.ai_thinking:
              self.setting_panel.undo_button.config(state=tk.DISABLED)
              self.setting_panel.redo_button.config(state=tk.DISABLED)
              return

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
            self.canvas.delete_debug()

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

    def set_game(self, game):
        self.game = game
        self.canvas.set_game(game)


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
    parser.add_argument("--black", type=str, default="Bob")
    parser.add_argument("--white", type=str, default="Alice")

    parser.add_argument("--board", type=str, help="Path to board file")

    parser.add_argument("--history", type=str, help="Path to move history file")

    parser.add_argument(
        "--history-until", type=int, help="index of history where you want to stop"
    )
    parser.add_argument("--use-undo", type=bool, default=False, help="set it as true to use undo/redo")

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
    app = GomokuGUI(root, args.black, args.white, history, args.use_undo)

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
