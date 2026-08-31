import math
import tkinter as tk

from src.screen_constant import BORDER_COLOR

SCORE_BAR_WIDTH = 24
SCORE_BAR_SCALE = 30_000  # higher = bar reacts more slowly to score changes
SCORE_TEXT_MARGIN = 26
SCORE_TEXT_FONT = ("Helvetica", 10, "bold")


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

        # Score sits on the leading side's end, like a chess eval bar
        if score >= 0:
            text_y, text_fill = SCORE_TEXT_MARGIN, "white"
        else:
            text_y, text_fill = self.height - SCORE_TEXT_MARGIN, "black"
        self.canvas.create_text(
            SCORE_BAR_WIDTH / 2,
            text_y,
            text=f"{score:+.0f}",
            fill=text_fill,
            font=SCORE_TEXT_FONT,
            angle=90,
        )
