import tkinter as tk
from tkinter import simpledialog
from lib_gomoku import Gomoku, MoveResult, get_ai_move
import argparse
import time

CELL_SIZE = 32
LABEL_PADDING = 10
BOARD_SIZE = 19
PADDING = 30

LIGHT_BACKGROUND = "#FAEBD7"
BORDER_COLOR = "#6f5c43"


class GomokuGUI:
    def __init__(self, root, player1, player2, history=None):
        self.root = root
        self.root.title("Gomoku")

        self.cell_size = CELL_SIZE
        self.board_size = BOARD_SIZE
        self.canvas_size = self.cell_size * (self.board_size - 1) + PADDING * 2

        self.game = Gomoku(size=self.board_size)

        # ===== MAIN LAYOUT =====
        self.main_frame = tk.Frame(root)
        self.main_frame.pack()

        self.left_frame = tk.Frame(self.main_frame)
        self.left_frame.pack(side="left")

        self.right_frame = tk.Frame(self.main_frame, padx=5)
        self.right_frame.pack(side="right", fill="y")
        self.right_frame.config(
            background=LIGHT_BACKGROUND,
            highlightbackground=BORDER_COLOR,
            highlightthickness=3,
        )

        # ===== CANVAS =====
        self.canvas = tk.Canvas(
            self.left_frame,
            width=self.canvas_size,
            height=self.canvas_size,
            bg="burlywood",
        )
        self.canvas.config(
            highlightbackground=BORDER_COLOR,
            highlightthickness=3,
        )
        self.canvas.pack()

        # ===== PLAYER NAMES =====
        self.player_names = {"X": player1, "O": player2}

        if player1 is None or player2 is None:
            self.ask_player_names()

        # ===== PLAYER BOXES =====
        self.player_frames = {}

        for p in ["X", "O"]:
            frame = tk.Frame(self.right_frame, padx=10, pady=10)
            frame.pack(fill="x", pady=5)

            # ⚫ / ⚪
            stone = "⚫" if p == "X" else "⚪"
            icon_label = tk.Label(
                frame, text=stone, font=("Arial", 16), background=LIGHT_BACKGROUND
            )
            icon_label.pack(side="left")

            # name
            name_label = tk.Label(
                frame,
                text=f"Player {p}",
                font=("Arial", 12),
                background=LIGHT_BACKGROUND,
            )
            name_label.pack(side="left", padx=5)

            # time
            time_label = tk.Label(
                frame, text="0ms", font=("Arial", 12), background=LIGHT_BACKGROUND
            )
            time_label.pack(side="right")

            self.player_frames[p] = {
                "frame": frame,
                "name": name_label,
                "time": time_label,
                "start_time": None,
            }
            self.player_frames[p]["frame"].config(
                background=LIGHT_BACKGROUND,
                highlightbackground=BORDER_COLOR,
                highlightthickness=1,
            )

        self.init_player_boxes()

        # ===== SETTINGS =====
        self.settings_frame = tk.LabelFrame(
            self.right_frame, text="Settings", padx=10, pady=10
        )
        self.settings_frame.pack(fill="x", pady=10)

        tk.Label(self.settings_frame, text="Ruleset").pack(anchor="w")
        self.ruleset_var = tk.StringVar(value="Standard")
        tk.OptionMenu(self.settings_frame, self.ruleset_var, "Standard", "Pro").pack(
            fill="x"
        )

        self.debug_var = tk.BooleanVar(value=False)
        tk.Checkbutton(
            self.settings_frame,
            text="Debug Mode",
            variable=self.debug_var,
            command=self.on_toggle_debug,
        ).pack(anchor="w")

        # ===== INIT BOARD =====
        # if history:
        #     for x, y in history:
        #         self.play_one_turn(x, y)

        self.draw_board(self.game.board)

        self.start_turn_timer()
        self.update_live_timer()

        self.canvas.bind("<Button-1>", self.handle_click)

    # ===== PLAYER SETUP =====
    def ask_player_names(self):
        self.player_names["X"] = (
            simpledialog.askstring("Player X", "Enter name for Player X") or "Player X"
        )
        self.player_names["O"] = (
            simpledialog.askstring("Player O", "Enter name for Player O") or "Player O"
        )

    def init_player_boxes(self):
        for p in ["X", "O"]:
            name = self.player_names[p]
            label = f"{name} (AI)" if name.lower() == "ai" else name
            self.player_frames[p]["name"].config(text=label)

    # ===== TIMER =====
    def start_turn_timer(self):
        p = self.game.current_player
        self.player_frames[p]["start_time"] = time.time()
        # print(f"{p} timer started, {self.player_frames[p]['start_time']}")

    def end_turn_timer(self):
        p = self.game.current_player
        start = self.player_frames[p]["start_time"]

        elapsed = (time.time() - start) * 1000
        self.player_frames[p]["time"].config(text=f"Time: {elapsed:.0f} ms")
        self.player_frames[p]["start_time"] = time.time()

    def update_live_timer(self):
        p = self.game.current_player
        start = self.player_frames[p]["start_time"]

        if start:
            elapsed = (time.time() - start) * 1000

            self.player_frames[p]["time"].config(text=f"Time: {elapsed:.0f} ms")

        self.root.after(100, self.update_live_timer)

    # ===== UI UPDATES =====
    def highlight_active_player(self):
        # print(f"highlight_active_player {self.game.current_player}")
        for p in ["X", "O"]:
            if p == self.game.current_player:
                self.player_frames[p]["frame"].config(
                    background=LIGHT_BACKGROUND,
                    highlightbackground=BORDER_COLOR,
                    highlightthickness=3,
                )
            else:
                self.player_frames[p]["frame"].config(
                    background=LIGHT_BACKGROUND,
                    highlightbackground=BORDER_COLOR,
                    highlightthickness=1,
                )

    # def update_alert_label(self, msg):
    #     self.alert_label.config(text=msg)

    # def update_ai_label(self, msg):
    #     self.ai_label.config(text=msg)

    def on_toggle_debug(self):
        print("Debug mode:", self.debug_var.get())

    # ===== DRAWING =====
    def draw_grid(self):
        for i in range(BOARD_SIZE):
            x = PADDING + i * CELL_SIZE
            y = PADDING + i * CELL_SIZE

            self.canvas.create_line(
                PADDING, y, PADDING + (BOARD_SIZE - 1) * CELL_SIZE, y
            )
            self.canvas.create_line(
                x, PADDING, x, PADDING + (BOARD_SIZE - 1) * CELL_SIZE
            )

            self.canvas.create_text(
                PADDING - LABEL_PADDING,
                y,
                text=str(i + 1),
                font=("Arial", 10),
                anchor="e",
                fill=BORDER_COLOR,
            )
            self.canvas.create_text(
                BOARD_SIZE * CELL_SIZE + LABEL_PADDING,
                y,
                text=str(i + 1),
                font=("Arial", 10),
                anchor="w",
                fill=BORDER_COLOR,
            )
            self.canvas.create_text(
                x,
                PADDING - LABEL_PADDING,
                text=chr(ord("A") + i),
                font=("Arial", 10),
                anchor="s",
                fill=BORDER_COLOR,
            )
            self.canvas.create_text(
                x,
                BOARD_SIZE * CELL_SIZE + LABEL_PADDING,
                text=chr(ord("A") + i),
                font=("Arial", 10),
                fill=BORDER_COLOR,
                anchor="n",
            )

    def draw_stone(self, x, y, player):
        color = "black" if player == "X" else "white"
        cx = PADDING + x * CELL_SIZE
        cy = PADDING + y * CELL_SIZE
        r = CELL_SIZE // 2 - 2
        self.canvas.create_oval(cx - r, cy - r, cx + r, cy + r, fill=color)

    def draw_board(self, board):
        self.canvas.delete("all")
        self.draw_grid()
        for y, row in enumerate(board):
            for x, cell in enumerate(row):
                if cell != ".":
                    self.draw_stone(x, y, cell)

    # ===== GAME FLOW =====
    def handle_click(self, event):
        x = round((event.x - PADDING) / CELL_SIZE)
        y = round((event.y - PADDING) / CELL_SIZE)
        self.play_one_turn(y, x)

        # assume that we are in human vs ai mode
        self.ai_play()

    def play_one_turn(self, x, y):
        result = self.game.is_valid_move(x, y)
        # self.update_alert_label("")

        if result == MoveResult.VALID:
            capture = self.game.handle_move(x, y)

            self.draw_board(self.game.board)

            winner = self.game.get_winner()
            if winner:
                self.finish_game(winner)
            else:
                self.change_turn()
                # self.start_turn_timer()
        # else:
        # self.update_alert_label(f"Invalid: {result.name}")

    def change_turn(self):
        self.end_turn_timer()

        self.game.switch_player()

        self.highlight_active_player()
        self.start_turn_timer()
        self.update_live_timer()

    def ai_play(self):
        start = time.time()
        # self.update_ai_label("AI thinking...")

        mv, _ = get_ai_move(self.game)
        if mv:
            x, y, _ = mv
            self.play_one_turn(x, y)

        # self.update_ai_label(f"AI time: {(time.time() - start):.3f}s")

    def finish_game(self, winner):
        self.canvas.create_text(
            self.canvas_size // 2,
            self.canvas_size // 2,
            text=f"{self.player_names[winner]} wins",
            fill="white",
            font=("Helvetica", 32, "bold"),
        )


# ===== MAIN =====
if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--black", type=str, default=None)
    parser.add_argument("--white", type=str, default=None)

    args = parser.parse_args()

    root = tk.Tk()
    app = GomokuGUI(root, args.black, args.white)
    root.mainloop()
