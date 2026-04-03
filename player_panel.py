import tkinter as tk
import time

from constants import (
    LIGHT_BACKGROUND,
    SELECT_BACKGROUND,
    BORDER_COLOR,
    NAME_FONT,
)


class PlayerPanel:
    def __init__(self, root, parent, player_name: str, is_X: bool, is_human: bool):
        self.root = root
        self.player_name = player_name
        self.start_time = None
        self.capture_count = 0

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
        stone = "⚫" if is_X else "⚪"
        self.icon_label = tk.Label(
            top_frame, text=stone, font=("Arial", 16), bg=LIGHT_BACKGROUND
        )
        self.icon_label.pack(side="left")

        # name
        self.name_label = tk.Label(
            top_frame,
            text=("AI" if not is_human else f"Player {player_name}"),
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

    # timer
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

    # highlight
    def hightlight_player(self):
        self.name_label.config(background=SELECT_BACKGROUND)

    def unhightlight_player(self):
        self.name_label.config(background=LIGHT_BACKGROUND)

    # capture
    def update_capture(self, count):
        if self.capture_count != count:
            self.capture_count = count
            for i in range(5):
                circle = self.capture_circles[i]
                if i < self.capture_count:
                    self.capture_canvas.itemconfig(circle, fill="black")
                else:
                    self.capture_canvas.itemconfig(circle, fill="")

    # update name
    def update_player_type(self, is_human: bool):
        self.name_label.config(
            text=("AI" if not is_human else f"Player {self.player_name}")
        )

    def reset_panel(self):
        self.time_label.config(text="0 ms")


class Player:
    def __init__(self, is_X: bool, name: str, is_human: bool, panel: PlayerPanel):
        self.is_X = is_X
        self.name = name
        self.is_human = is_human
        self.panel = panel
