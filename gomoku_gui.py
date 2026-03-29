import tkinter as tk
from tkinter import simpledialog
from lib_gomoku import Gomoku, MoveResult, get_ai_move
import argparse
import time
import threading

CELL_SIZE = 32
LABEL_PADDING = 10
BOARD_SIZE = 19
PADDING = 30

LIGHT_BACKGROUND = "#FAEBD7"
SELECT_BACKGROUND = "#A68A64"
BORDER_COLOR = "#6f5c43"

LABEL_FONT = "Phosphate"
NAME_FONT = "Rockwell"
TIMER_FONT = "Skia"


class Player:
    def __init__(self, name: str, is_human: bool):
        self.name = name
        self.is_human = is_human


class GomokuGUI:
    def __init__(self, root, player1, player2, history=None):
        self.root = root
        self.root.title("Gomoku")

        self.cell_size = CELL_SIZE
        self.board_size = BOARD_SIZE
        self.canvas_size = self.cell_size * (self.board_size - 1) + PADDING * 2

        self.game = Gomoku(size=self.board_size)

        # ===== MAIN LAYOUT =====
        self.main_frame = tk.Frame(
            root,
            highlightbackground=BORDER_COLOR,
            highlightthickness=2,
        )
        self.main_frame.pack()

        self.left_frame = tk.Frame(self.main_frame)
        self.left_frame.pack(side="left")

        self.right_frame = tk.Frame(self.main_frame, width=250)
        self.right_frame.pack(side="right", fill="y")
        self.right_frame.config(
            background=LIGHT_BACKGROUND,
            highlightbackground=BORDER_COLOR,
            highlightthickness=2,
        )
        self.right_frame.pack_propagate(False)

        # ===== CANVAS =====
        self.canvas = tk.Canvas(
            self.left_frame,
            width=self.canvas_size,
            height=self.canvas_size,
            bg="burlywood",
        )
        self.canvas.config(
            highlightbackground=BORDER_COLOR,
            highlightthickness=2,
        )
        self.canvas.pack()
        self.draw_grid()

        # ===== PLAYER =====
        # if player1 is None or player2 is None:
        #     self.ask_player_names()

        self.players = {"X": Player(player1, True), "O": Player(player2, True)}

        # ===== PLAYER BOXES =====
        self.player_frames = {}

        for p in ["X", "O"]:
            frame = tk.Frame(self.right_frame, padx=10, pady=10)
            frame.pack(fill="x")

            top_frame = tk.Frame(frame, bg=LIGHT_BACKGROUND)
            top_frame.pack(fill="x")

            bottom_frame = tk.Frame(frame, bg=LIGHT_BACKGROUND)
            bottom_frame.pack(fill="x", pady=(5, 0))

            # ⚫ / ⚪
            stone = "⚫" if p == "X" else "⚪"
            icon_label = tk.Label(
                top_frame, text=stone, font=("Arial", 16), background=LIGHT_BACKGROUND
            )
            icon_label.pack(side="left")

            # name
            name_label = tk.Label(
                top_frame,
                text=(
                    f"Player {self.players[p].name}"
                    if self.players[p].is_human
                    else "AI"
                ),
                font=(NAME_FONT, 12),
                background=LIGHT_BACKGROUND,
            )
            name_label.pack(side="left", padx=5)

            # time
            time_label = tk.Label(
                top_frame,
                text="0 ms",
                font=(TIMER_FONT, 12),
                background=LIGHT_BACKGROUND,
            )
            time_label.pack(side="right")

            # capture
            CAPTURE_SIZE = 20
            CAPTURE_GAP = 28

            capture_canvas = tk.Canvas(
                bottom_frame,
                width=5 * (CAPTURE_SIZE + CAPTURE_GAP),
                height=CAPTURE_SIZE + 2,
                bg=LIGHT_BACKGROUND,
                highlightthickness=0,
            )
            capture_canvas.pack(side="left", padx=3, pady=10)

            circles = []

            for i in range(5):
                x = i * (CAPTURE_SIZE + CAPTURE_GAP)

                circle = capture_canvas.create_oval(
                    x,
                    0,
                    x + CAPTURE_SIZE,
                    CAPTURE_SIZE,
                    outline="gray40",
                    width=1,
                    fill="",
                )
                circles.append(circle)

            self.player_frames[p] = {
                "frame": frame,
                "name": name_label,
                "time": time_label,
                "start_time": None,
                "captures": 0,
                "capture_circles": circles,
                "capture_canvas": capture_canvas,
            }
            self.player_frames[p]["frame"].config(
                background=LIGHT_BACKGROUND,
                highlightbackground=BORDER_COLOR,
                highlightthickness=1,
            )

        # ===== SETTINGS =====
        self.settings_frame = tk.LabelFrame(
            self.right_frame,
            padx=10,
            pady=10,
            bg=LIGHT_BACKGROUND,
            highlightbackground=BORDER_COLOR,
            highlightthickness=1,
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

        # ===== INIT TIMER =====
        self.highlight_active_player()
        self.start_turn_timer()
        self.update_live_timer()

        self.canvas.bind("<Button-1>", self.handle_click)
        self.canvas.bind("<Motion>", self.handle_hover)

    # ===== PLAYER SETUP =====
    # def ask_player_names(self):
    #     self.player_names["X"] = (
    #         simpledialog.askstring("Player X", "Enter name for Player X") or "Player X"
    #     )
    #     self.player_names["O"] = (
    #         simpledialog.askstring("Player O", "Enter name for Player O") or "Player O"
    #     )

    # ===== TIMER =====
    def start_turn_timer(self):
        p = self.game.current_player
        self.player_frames[p]["start_time"] = time.time()

    def end_turn_timer(self):
        p = self.game.current_player
        start = self.player_frames[p]["start_time"]

        elapsed = (time.time() - start) * 1000
        self.player_frames[p]["time"].config(text=f"{elapsed:.0f} ms")
        self.player_frames[p]["start_time"] = time.time()

    def update_live_timer(self):
        p = self.game.current_player
        start = self.player_frames[p]["start_time"]

        if start:
            elapsed = (time.time() - start) * 1000

            self.player_frames[p]["time"].config(text=f"{elapsed:.0f} ms")

        self.root.after(100, self.update_live_timer)

    def on_toggle_debug(self):
        print("Debug mode:", self.debug_var.get())

    # ===== DRAWING =====
    def highlight_active_player(self):
        for p in ["X", "O"]:
            if p == self.game.current_player:
                self.player_frames[p]["name"].config(
                    background=SELECT_BACKGROUND,
                )
            else:
                self.player_frames[p]["name"].config(
                    background=LIGHT_BACKGROUND,
                )

    def draw_grid(self):
        DOT_RADIUS = 4
        points = [3, 9, 15]

        for row in points:
            for col in points:
                x = PADDING + col * CELL_SIZE
                y = PADDING + row * CELL_SIZE

                self.canvas.create_oval(
                    x - DOT_RADIUS,
                    y - DOT_RADIUS,
                    x + DOT_RADIUS,
                    y + DOT_RADIUS,
                    fill="black",
                    tags="grid",
                )
        for i in range(BOARD_SIZE):
            x = PADDING + i * CELL_SIZE
            y = PADDING + i * CELL_SIZE

            self.canvas.create_line(
                PADDING, y, PADDING + (BOARD_SIZE - 1) * CELL_SIZE, y, tags="grid"
            )
            self.canvas.create_line(
                x, PADDING, x, PADDING + (BOARD_SIZE - 1) * CELL_SIZE, tags="grid"
            )

            self.canvas.create_text(
                PADDING - LABEL_PADDING,
                y,
                text=str(i + 1),
                font=(LABEL_FONT, 10),
                anchor="e",
                fill=BORDER_COLOR,
                tags="grid",
            )
            self.canvas.create_text(
                BOARD_SIZE * CELL_SIZE + LABEL_PADDING,
                y,
                text=str(i + 1),
                font=(LABEL_FONT, 10),
                anchor="w",
                fill=BORDER_COLOR,
                tags="grid",
            )
            self.canvas.create_text(
                x,
                PADDING - LABEL_PADDING,
                text=chr(ord("A") + i),
                font=(LABEL_FONT, 10),
                anchor="s",
                fill=BORDER_COLOR,
                tags="grid",
            )
            self.canvas.create_text(
                x,
                BOARD_SIZE * CELL_SIZE + LABEL_PADDING,
                text=chr(ord("A") + i),
                font=(LABEL_FONT, 10),
                fill=BORDER_COLOR,
                anchor="n",
                tags="grid",
            )

    def draw_stones(self, board):
        self.canvas.delete("stone")
        for y, row in enumerate(board):
            for x, cell in enumerate(row):
                if cell != ".":
                    self.draw_stone(x, y, cell)

    def draw_stone(self, x, y, player):
        color = "black" if player == "X" else "white"
        cx = PADDING + x * CELL_SIZE
        cy = PADDING + y * CELL_SIZE
        r = CELL_SIZE // 2 - 2
        self.canvas.create_oval(
            cx - r, cy - r, cx + r, cy + r, fill=color, tags="stone"
        )

    def draw_last_move(self, row, col):
        cx = PADDING + row * CELL_SIZE
        cy = PADDING + col * CELL_SIZE
        r = CELL_SIZE // 2 - 12
        self.canvas.delete("last-move")

        self.last_move_marker = self.canvas.create_oval(
            cx - r,
            cy - r,
            cx + r,
            cy + r,
            fill="red",
            outline="red",
            tags="last-move",
        )

    def update_captures(self, player, count=1):
        data = self.player_frames[player]
        data["captures"] += count

        total = data["captures"]
        circles = data["capture_circles"]
        canvas = data["capture_canvas"]

        fill_color = "black" if player == "X" else "white"

        for i, c in enumerate(circles):
            if i < total:
                canvas.itemconfig(c, fill=fill_color)
            else:
                canvas.itemconfig(c, fill="")

    # ===== HANDLE INPUT ====
    def handle_hover(self, event):
        x = round((event.x - PADDING) / CELL_SIZE)
        y = round((event.y - PADDING) / CELL_SIZE)

        self.canvas.delete("hover")

        if not (0 <= x < BOARD_SIZE and 0 <= y < BOARD_SIZE):
            return

        cx = PADDING + x * CELL_SIZE
        cy = PADDING + y * CELL_SIZE
        r = CELL_SIZE // 2 - 2
        color = "#333333" if self.game.current_player == "X" else "#DDDDDD"

        self.canvas.create_oval(
            cx - r,
            cy - r,
            cx + r,
            cy + r,
            fill=color,
            stipple="gray50",
            width=1,
            tags="hover",
        )

    def handle_click(self, event):
        if self.players[self.game.current_player].is_human:
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

            self.draw_stones(self.game.board)

            winner = self.game.get_winner()
            self.draw_last_move(y, x)
            if winner:
                self.finish_game(winner)
            else:
                self.change_turn()
                if not self.players[self.game.current_player].is_human:
                    self.ai_play()
        # else:
        # self.update_alert_label(f"Invalid: {result.name}")

    def change_turn(self):
        self.end_turn_timer()

        self.game.switch_player()

        self.highlight_active_player()
        self.start_turn_timer()
        self.update_live_timer()

    def ai_play(self):
        def run_ai():
            mv, _ = get_ai_move(self.game)
            if mv:
                x, y, _ = mv
                self.root.after(0, lambda: self.play_one_turn(x, y))

        threading.Thread(target=run_ai, daemon=True).start()

    def finish_game(self, winner):
        self.canvas.create_text(
            self.canvas_size // 2,
            self.canvas_size // 2,
            text=f"{self.players[winner].name} wins",
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
