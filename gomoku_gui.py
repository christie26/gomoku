import tkinter as tk
from tkinter import simpledialog
from faster_functions import Gomoku, MoveResult, get_ai_move
import argparse
import time

CELL_SIZE = 35
BOARD_SIZE = 19
PADDING = 30
LABEL_PADDING = 17


class GomokuGUI:
    """
    A GUI class for playing Gomoku using Tkinter. Supports human and AI players,
    handles user interaction, board drawing, turn management, and displaying results.
    """

    def __init__(self, root, player1, player2, history=None):
        """
        Initialize the GUI, canvas, labels, and player info.

        Args:
            root: The Tkinter root window.
            player1 (str or None): Name of player X, or None to ask.
            player2 (str or None): Name of player O, or None to ask.
        """
        self.root = root
        self.root.title("Gomoku")
        self.cell_size = CELL_SIZE
        self.board_size = BOARD_SIZE
        self.canvas_size = self.cell_size * (self.board_size - 1) + PADDING * 2

        # Create board and canvas
        self.game = Gomoku(size=self.board_size)

        self.canvas = tk.Canvas(
            root, width=self.canvas_size, height=self.canvas_size, bg="burlywood"
        )
        self.canvas.pack(pady=0, padx=0)

        self.info_label = tk.Label(root, text="", font=("Arial", 20))
        self.info_label.pack(pady=5)

        self.turn_label = tk.Label(root, text="", font=("Arial", 20))
        self.turn_label.pack(pady=5)

        self.alert_label = tk.Label(root, text="", font=("Arial", 20))
        self.alert_label.pack(pady=5)

        self.ai_label = tk.Label(root, text="", font=("Arial", 20))
        self.ai_label.pack(pady=5)

        # Player names
        self.player_names = {"X": player1, "O": player2}

        if player1 == None or player2 == None:
            self.ask_player_names()

        # if user import history file, play those move first
        if history:
            for x, y in history:
                self.play_one_turn(x, y)

        self.draw_board(self.game.board)
        self.update_info_label()
        self.update_turn_label()

        self.root.update_idletasks()
        self.ai_play()
        self.root.update_idletasks()

        self.canvas.bind("<Button-1>", self.handle_click)

    def ask_player_names(self):
        self.player_names["X"] = (
            simpledialog.askstring("Player X", "Enter name for Player X (black):")
            or "Player X"
        )
        self.player_names["O"] = (
            simpledialog.askstring("Player O", "Enter name for Player O (white):")
            or "Player O"
        )

    def update_info_label(self):
        self.info_label.config(
            text=(
                f"⚫ {self.player_names['X']} (captures: {self.game.capture_count['X']}) "
                f"vs {self.player_names['O']} (captures: {self.game.capture_count['O']}) ⚪"
            )
        )

    def update_turn_label(self):
        turn_name = self.player_names[self.game.current_player]
        turn_color = "⚫" if self.game.current_player == "X" else "⚪"
        self.turn_label.config(text=f"{turn_name}'s Turn {turn_color}")

    def update_alert_label(self, message):
        self.alert_label.config(text=message)

    def update_ai_label(self, message):
        self.ai_label.config(text=message)

    def draw_grid(self):
        for i in range(BOARD_SIZE):
            x = PADDING + i * CELL_SIZE
            y = PADDING + i * CELL_SIZE

            # Grid lines
            self.canvas.create_line(
                PADDING, y, PADDING + (BOARD_SIZE - 1) * CELL_SIZE, y
            )
            self.canvas.create_line(
                x, PADDING, x, PADDING + (BOARD_SIZE - 1) * CELL_SIZE
            )

            # Row labels (1–19)
            self.canvas.create_text(
                PADDING - LABEL_PADDING,
                y,
                text=str(i + 1),
                font=("Arial", 10),
                anchor="e",
            )
            self.canvas.create_text(
                BOARD_SIZE * CELL_SIZE + LABEL_PADDING,
                y,
                text=str(i + 1),
                font=("Arial", 10),
                anchor="w",
            )

            # Column labels (A–S)
            self.canvas.create_text(
                x,
                PADDING - LABEL_PADDING,
                text=chr(ord("A") + i),
                font=("Arial", 10),
                anchor="s",
            )
            self.canvas.create_text(
                x,
                BOARD_SIZE * CELL_SIZE + LABEL_PADDING,
                text=chr(ord("A") + i),
                font=("Arial", 10),
                anchor="n",
            )

    def draw_stone(self, x, y, player):
        color = "black" if player == "X" else "white"
        cx = PADDING + x * CELL_SIZE
        cy = PADDING + y * CELL_SIZE
        r = CELL_SIZE // 2 - 2
        self.canvas.create_oval(cx - r, cy - r, cx + r, cy + r, fill=color)

    def draw_possible_stone(self, x, y, player, number, selected):
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
            cx, cy, text=number_str, fill=text_color, font=("Arial", font_size, "bold")
        )

    def finish_game(self, winner):
        # # Create a toplevel window to act as overlay
        # overlay = tk.Toplevel(root)
        #
        # overlay.geometry(
        #     f"{self.canvas_size}x{self.canvas_size}+{root.winfo_rootx() + self.canvas.winfo_x()}+{root.winfo_rooty() + self.canvas.winfo_y()}"
        # )
        # overlay.overrideredirect(True)
        # overlay.attributes("-topmost", True)
        # overlay.attributes("-alpha", 0.3)  # 30% opaque (i.e. 70% transparent)
        # overlay.configure(bg="black")
        #
        # def update_overlay_position():
        #     overlay.geometry(
        #         f"{self.canvas_size}x{self.canvas_size}+{root.winfo_rootx() + self.canvas.winfo_x()}+{root.winfo_rooty() + self.canvas.winfo_y()}"
        #     )
        #     root.after(50, update_overlay_position)
        #
        # update_overlay_position()

        # Display the result text in the center
        self.result_text = self.canvas.create_text(
            self.canvas_size // 2,
            self.canvas_size // 2,
            text=f"{self.player_names[winner]} wins",
            fill="white",
            font=("Helvetica", 32, "bold"),
        )

    def draw_board(self, board):
        self.canvas.delete("all")
        self.draw_grid()
        for y, row in enumerate(board):
            for x, cell in enumerate(row):
                if cell != ".":
                    self.draw_stone(x, y, cell)

    def handle_click(self, event):
        x = event.x // CELL_SIZE
        y = event.y // CELL_SIZE
        self.play_one_turn(y, x)  # note: board is row (y), col (x)
        self.root.update_idletasks()
        self.ai_play()
        self.root.update_idletasks()

    def play_one_turn(self, x, y) -> int:
        result = self.game.is_valid_move(x, y)
        self.update_alert_label("")
        if result == MoveResult.VALID:
            print(
                f"{self.player_names[self.game.current_player]} played {chr(ord('A') + y)}{x + 1}"
            )
            capture = self.game.handle_move(x, y)
            if capture:
                self.update_info_label()

                self.update_alert_label(
                    f"{self.player_names[self.game.current_player]} captures {self.player_names[self.game.opponent_player]}"
                )
            self.draw_board(self.game.board)
            winner = self.game.get_winner()
            if winner != None:
                self.finish_game(winner)
            else:
                self.game.switch_player()
                self.update_turn_label()
        else:
            self.update_alert_label(
                f"Invalid move: {result.name.replace('_', ' ').title()}"
            )
        return result

    def ai_play(self):
        start_time = time.time()

        self.update_ai_label("AI is thinking")
        # print("AI is thinking")

        mv, moves = get_ai_move(self.game)
        # print(f"mv: {mv}")
        # print(f"moves: {moves}")

        if mv is None:
            print(f"Found no valid moves: {mv} - {moves}")

        x, y, score = mv

        ai_time = time.time() - start_time
        self.update_ai_label(f"AI played in {ai_time:.4f}s")
        # print(
        #     f"AI chose to play {mv} in {ai_time:.4f}s out of {len(moves)} moves-------------------------"
        # )
        # print(self.game.print_state())

        for m in moves:
            x1, y1, score1 = m
            selected = x == x1 and y == y1
            self.draw_possible_stone(y1, x1, "O", score1, selected)

        # self.play_one_turn(x, y)


def load_and_validate_board(filepath):
    with open(filepath, "r") as f:
        lines = [line.strip() for line in f if line.strip()]

    if len(lines) != 19 or any(len(row) != 19 for row in lines):
        raise ValueError("Board must be 19x19")

    valid_symbols = {".", "X", "O"}
    board = []
    count = {"X": 0, "O": 0}

    for row in lines:
        if any(c not in valid_symbols for c in row):
            raise ValueError("Board can only contain '.', 'X', or 'O'")
        for c in row:
            if c in count:
                count[c] += 1
        board.append(list(row))

    current_player = "X"

    return board, current_player


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


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Simple Gomoku Game")

    parser.add_argument("--size", type=int, default=19, help="Board size (default: 19)")
    parser.add_argument(
        "--black", type=str, default=None, help="Name of black stone player (X)"
    )
    parser.add_argument(
        "--white", type=str, default=None, help="Name of white stone player (O)"
    )
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
        app.update_turn_label()

    root.mainloop()
