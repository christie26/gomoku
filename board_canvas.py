import tkinter as tk
from lib_gomoku import Gomoku, MoveResult, get_ai_move

CELL_SIZE = 32
LABEL_PADDING = 10
BOARD_SIZE = 19
PADDING = 30

LIGHT_BACKGROUND = "#FAEBD7"
SELECT_BACKGROUND = "#A68A64"
BORDER_COLOR = "#6f5c43"

NAME_FONT = "Rockwell"


class BoardCanvas:
    def __init__(self, parent, canvas_size, on_click):
        self.canvas_size = canvas_size
        self.game = None
        self.is_playing = False

        self.canvas = tk.Canvas(
            parent,
            width=canvas_size,
            height=canvas_size,
            highlightbackground=BORDER_COLOR,
            highlightthickness=2,
            bg="burlywood",
        )
        self.canvas.pack()

        self.canvas.bind("<Button-1>", on_click)
        self.canvas.bind("<Motion>", self.handle_hover)

        self.hover_point = None

        # overlay
        self.overlay = self.canvas.create_rectangle(
            0,
            0,
            canvas_size + 5,
            canvas_size + 5,
            fill="gray",
            stipple="gray50",
            outline="",
            tags="overlay",
        )
        self.draw_grid()

    # ===== DRAW =====
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
                font=(NAME_FONT, 14),
                anchor="e",
                fill=BORDER_COLOR,
                tags="grid",
            )
            self.canvas.create_text(
                BOARD_SIZE * CELL_SIZE + LABEL_PADDING,
                y,
                text=str(i + 1),
                font=(NAME_FONT, 14),
                anchor="w",
                fill=BORDER_COLOR,
                tags="grid",
            )
            self.canvas.create_text(
                x,
                PADDING - LABEL_PADDING,
                text=chr(ord("A") + i),
                font=(NAME_FONT, 14),
                anchor="s",
                fill=BORDER_COLOR,
                tags="grid",
            )
            self.canvas.create_text(
                x,
                BOARD_SIZE * CELL_SIZE + LABEL_PADDING,
                text=chr(ord("A") + i),
                font=(NAME_FONT, 14),
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

    # ==== LAST MOVE ====
    def draw_last_move(self, x, y):
        cx = PADDING + x * CELL_SIZE
        cy = PADDING + y * CELL_SIZE
        r = CELL_SIZE // 2 - 12

        self.canvas.delete("last-move")
        self.canvas.create_oval(
            cx - r, cy - r, cx + r, cy + r, fill="red", outline="red", tags="last-move"
        )

    def remove_last_move(self):
        self.canvas.delete("last-move")

    # ==== DEBUG ====
    def draw_debug(self, moves, best_move=None):
        for m in moves:
            x1, y1, score1 = m

            selected = False
            if best_move:
                bx, by, _ = best_move
                selected = bx == x1 and by == y1

            self.draw_debug_stone(y1, x1, "O", score1, selected)

    def draw_debug_stone(self, x, y, player, number, selected):
        color = "black" if player == "X" else "white"
        cx = PADDING + x * CELL_SIZE
        cy = PADDING + y * CELL_SIZE
        r = CELL_SIZE // 2 - 2

        # Create stone with 50% opacity
        outline_color = "red" if selected else ""
        self.canvas.create_oval(
            cx - r,
            cy - r,
            cx + r,
            cy + r,
            fill=color,
            stipple="gray50",
            outline=outline_color,
            tags="debug",
        )
        # Calculate appropriate font size based on stone size and number length
        number_str = str(number)
        # Base font size proportional to stone radius
        base_font_size = max(6, r // 2)

        # Reduce font size for longer numbers
        if len(number_str) == 1:
            font_size = base_font_size
        elif len(number_str) == 2:
            font_size = max(6, int(base_font_size))
        else:  # 3+ digits
            font_size = max(6, int(base_font_size))

        # Add numeric text in the center
        text_color = "white" if player == "X" else "black"
        self.canvas.create_text(
            cx,
            cy,
            text=number_str,
            fill=text_color,
            font=("Arial", font_size, "bold"),
            tags="debug",
        )

    def remove_debug(self):
        self.canvas.delete("debug")

    # ===== HINT =====
    def draw_hint(self, best_move):
        x, y, _ = best_move
        cx = PADDING + x * CELL_SIZE
        cy = PADDING + y * CELL_SIZE
        r = CELL_SIZE // 2 - 2

        self.canvas.create_oval(
            cx - r,
            cy - r,
            cx + r,
            cy + r,
            fill="",
            stipple="gray50",
            outline="red",
            tags="hint",
        )

    def remove_hint(self):
        self.canvas.delete("hint")

    # ===== HOVER =====
    def handle_hover(self, event):
        if self.is_playing:
            x = round((event.x - PADDING) / CELL_SIZE)
            y = round((event.y - PADDING) / CELL_SIZE)

            if (x, y) == self.hover_point:
                return

            self.hover_point = (x, y)
            self.canvas.delete("hover")

            if (
                not (0 <= x < BOARD_SIZE and 0 <= y < BOARD_SIZE)
                or self.game.board[y][x] != "."
            ):
                return

            cx = PADDING + x * CELL_SIZE
            cy = PADDING + y * CELL_SIZE
            r = CELL_SIZE // 2 - 2
            if self.game.is_valid_move(y, x) == MoveResult.DOUBLE_THREE:
                color = "#FF4D4D"
            else:
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

    def remove_hover(self, event=None):
        self.hover_point = None
        self.canvas.delete("hover")

    # ===== OVERLAY =====
    def show_overlay(self):
        self.canvas.itemconfig("overlay", state="normal")

    def hide_overlay(self):
        self.canvas.itemconfig("overlay", state="hidden")

    # ===== TEXT =====
    def show_winner(self, text):
        self.canvas.create_rectangle(
            self.canvas_size // 2 - 100,
            self.canvas_size // 2 - 20,
            self.canvas_size // 2 + 100,
            self.canvas_size // 2 + 20,
            fill=BORDER_COLOR,
            stipple="gray50",
            outline="",
            tags="message",
        )

        self.canvas.create_text(
            self.canvas_size // 2,
            self.canvas_size // 2,
            text=text,
            fill="white",
            font=(NAME_FONT, 32, "bold"),
            tags="message",
        )

    # ===== RESET ====
    def reset_board(self, is_playing: bool):
        self.is_playing = is_playing

        if is_playing:
            self.canvas.delete("stone")
            self.canvas.delete("last-move")
            self.canvas.delete("message")
            self.canvas.itemconfig(self.overlay, state="hidden")
        else:
            self.canvas.itemconfig(self.overlay, state="normal")

    def set_game(self, game):
        self.game = game

    def show_capture(self, positions):
        """
        positions: [(x, y), ...]
        """
        for y, x in positions:
            cx = PADDING + x * CELL_SIZE
            cy = PADDING + y * CELL_SIZE
            r = CELL_SIZE // 2

            self.canvas.create_oval(
                cx - r,
                cy - r,
                cx + r,
                cy + r,
                outline="red",
                width=3,
                tags="capture",
            )

        self.canvas.after(700, lambda: self.canvas.delete("capture"))
