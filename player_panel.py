import tkinter as tk
import time

LIGHT_BACKGROUND = "#FAEBD7"
SELECT_BACKGROUND = "#A68A64"
BORDER_COLOR = "#6f5c43"

NAME_FONT = "Rockwell"


class Player:
    def __init__(self, is_X: bool, name: str, is_human: bool):
        self.is_X = is_X
        self.name = name
        self.is_human = is_human


class PlayerPanel:
    def __init__(self, root, parent, player: Player):
        self.root = root
        self.player = player
        self.start_time = None
        self.captures = 0

        self.frame = tk.Frame(
            parent,
            padx=10,
            pady=10,
            bg=LIGHT_BACKGROUND,
            highlightbackground=BORDER_COLOR,
            highlightthickness=1,
        )
        self.frame.pack(fill="x")

        # --- top ---
        top_frame = tk.Frame(self.frame, bg=LIGHT_BACKGROUND)
        top_frame.pack(fill="x")

        bottom_frame = tk.Frame(self.frame, bg=LIGHT_BACKGROUND)
        bottom_frame.pack(fill="x", pady=(5, 0))

        # icon
        stone = "⚫" if player.is_X else "⚪"
        self.icon_label = tk.Label(
            top_frame, text=stone, font=("Arial", 16), bg=LIGHT_BACKGROUND
        )
        self.icon_label.pack(side="left")

        # name
        self.name_label = tk.Label(
            top_frame,
            text=("AI" if not player.is_human else f"Player {player.name}"),
            font=(NAME_FONT, 14),
            bg=LIGHT_BACKGROUND,
        )
        self.name_label.pack(side="left", padx=5)

        # time
        self.time_label = tk.Label(
            top_frame,
            text="0 ms",
            font=(NAME_FONT, 12),
            bg=LIGHT_BACKGROUND,
        )
        self.time_label.pack(side="right")

        # --- capture ---
        self.capture_canvas = tk.Canvas(
            bottom_frame,
            width=5 * 28,
            height=22,
            bg=LIGHT_BACKGROUND,
            highlightthickness=0,
        )
        self.capture_canvas.pack(side="left", padx=3, pady=10)

        self.capture_circles = []

        for i in range(5):
            x = i * 28
            circle = self.capture_canvas.create_oval(
                x,
                0,
                x + 20,
                20,
                outline="gray40",
                width=1,
                fill="",
            )
            self.capture_circles.append(circle)

    # -------------------------
    # Timer
    # -------------------------
    def start_timer(self):
        self.start_time = time.time()
        self.update_live_timer()

    def stop_timer(self):
        if self.start_time:
            elapsed = (time.time() - self.start_time) * 1000
            self.time_label.config(text=f"{elapsed:.0f} ms")
            self.start_time = None

    def update_live_timer(self):
        if self.start_time:
            elapsed = (time.time() - self.start_time) * 1000
            self.time_label.config(text=f"{elapsed:.0f} ms")
            self.root.after(100, self.update_live_timer)

    # -------------------------
    # Hightlight
    # -------------------------
    def hightlight_player(self):
        self.name_label.config(background=SELECT_BACKGROUND)

    def unhightlight_player(self):
        self.name_label.config(background=LIGHT_BACKGROUND)

    # -------------------------
    # Capture
    # -------------------------
    def add_capture(self):
        if self.captures >= 5:
            return

        circle = self.capture_circles[self.captures]
        self.capture_canvas.itemconfig(circle, fill="black")
        self.captures += 1
