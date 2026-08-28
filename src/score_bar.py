import math
import tkinter as tk

from src.screen_constant import BORDER_COLOR

SCORE_BAR_WIDTH = 24
SCORE_BAR_SCALE = 3000  # higher = bar reacts more slowly to score changes


class ScoreBar:
    def __init__(self, parent, height):
        self.height = height
        self.canvas = tk.Canvas(
            parent,
            width=SCORE_BAR_WIDTH,
            height=height,
            highlightbackground=BORDER_COLOR,
            highlightthickness=1,
        )
        self.canvas.pack(side="left", fill="y")
        self.update_score(0)

    def update_score(self, score):
        # score > 0 favors black, score < 0 favors white
        black_ratio = 0.5 + 0.5 * math.tanh(score / SCORE_BAR_SCALE)
        split_y = self.height * black_ratio

        self.canvas.delete("all")
        self.canvas.create_rectangle(
            0, 0, SCORE_BAR_WIDTH, split_y, fill="black", outline=""
        )
        self.canvas.create_rectangle(
            0, split_y, SCORE_BAR_WIDTH, self.height, fill="white", outline=""
        )
