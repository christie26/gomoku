# gomoku_gui.py
import tkinter as tk
from tkinter import simpledialog
from gomoku import Gomoku, MoveResult
import argparse

CELL_SIZE = 30
BOARD_SIZE = 19
PADDING = 20

class GomokuGUI:
    def __init__(self, root, player1, player2):
        self.root = root
        self.root.title("Gomoku")
        self.cell_size = 30
        self.board_size = 19
        self.canvas_size = self.cell_size * self.board_size + 20

        # Create board and canvas
        self.game = Gomoku(size=self.board_size)
        
        self.canvas = tk.Canvas(root, width=self.canvas_size, height=self.canvas_size, bg="burlywood")
        self.canvas.pack(pady=10)

        self.info_label = tk.Label(root, text="", font=("Arial", 15))
        self.info_label.pack(pady=5)

        self.turn_label = tk.Label(root, text="", font=("Arial", 15))
        self.turn_label.pack(pady=5)

        self.alert_label= tk.Label(root, text="", font=("Arial", 15))
        self.alert_label.pack(pady=5)

        # Player names
        self.player_names = {
            'X': player1,
            'O': player2
        }

        if player1 == None or player2 == None:
          self.ask_player_names()
  
        self.draw_board(self.game.board)
        self.update_info_label()
        self.update_turn_label()

        self.canvas.bind("<Button-1>", self.handle_click)

    def ask_player_names(self):
        self.player_names['X'] = simpledialog.askstring("Player X", "Enter name for Player X (black):") or "Player X"
        self.player_names['O'] = simpledialog.askstring("Player O", "Enter name for Player O (white):") or "Player O"
 
    def update_info_label(self):
        self.info_label.config(
            text=f"⚫ {self.player_names['X']}  vs {self.player_names['O']} ⚪"
        )
  
    def update_turn_label(self):
        turn_name = self.player_names[self.game.current_player]
        turn_color = "⚫" if self.game.current_player == 'X' else "⚪"
        self.turn_label.config(
            text=f"{turn_name}'s Turn {turn_color}"
        )
        

    def draw_grid(self):
        for i in range(BOARD_SIZE):
            x = PADDING + i * CELL_SIZE
            y = PADDING + i * CELL_SIZE

            # Grid lines
            self.canvas.create_line(PADDING, y, PADDING + (BOARD_SIZE - 1) * CELL_SIZE, y)
            self.canvas.create_line(x, PADDING, x, PADDING + (BOARD_SIZE - 1) * CELL_SIZE)

            # Row labels (A–S)
            self.canvas.create_text(PADDING - 5, y, text=str(i), font=("Arial", 10), anchor='e')

            # Column labels (1–19)
            self.canvas.create_text(x, PADDING - 5, text=str(i), font=("Arial", 10), anchor='s')

    def draw_stone(self, x, y, player):
        color = 'black' if player == 'X' else 'white'
        cx = PADDING + x * CELL_SIZE
        cy = PADDING + y * CELL_SIZE
        r = CELL_SIZE // 2 - 2
        self.canvas.create_oval(cx - r, cy - r, cx + r, cy + r, fill=color)


    def draw_board(self, board):
        self.canvas.delete("all")
        self.draw_grid()
        for y, row in enumerate(board):
            for x, cell in enumerate(row):
                if cell != '.':
                    self.draw_stone(x, y, cell)

    def handle_click(self, event):
        x = event.x // CELL_SIZE
        y = event.y // CELL_SIZE
        result, capture = self.game.handle_move(y, x)  # note: board is row (y), col (x)
        self.alert_label.config(text="")
        if result == MoveResult.VALID:
            if capture:
              self.alert_label.config(text=f"{self.player_names[self.game.current_player]} captures {self.player_names[self.game.opponent_player]}")
            self.draw_board(self.game.board)
            if self.game.check_winner():
                self.alert_label.config(text=f"Player {self.player_names[self.game.current_player]} wins!")
                self.canvas.unbind("<Button-1>")
            else:
                self.game.switch_player()
                self.update_turn_label()
        else:
            self.alert_label.config(text=f"Invalid move: {result.name.replace('_', ' ').title()}")

def load_and_validate_board(filepath):
    with open(filepath, 'r') as f:
        lines = [line.strip() for line in f if line.strip()]

    if len(lines) != 19 or any(len(row) != 19 for row in lines):
        raise ValueError("Board must be 19x19")

    valid_symbols = {'.', 'X', 'O'}
    board = []
    count = {'X': 0, 'O': 0}

    for row in lines:
        if any(c not in valid_symbols for c in row):
            raise ValueError("Board can only contain '.', 'X', or 'O'")
        for c in row:
            if c in count:
                count[c] += 1
        board.append(list(row))

    if abs(count['X'] - count['O']) > 1:
        raise ValueError("Number of X and O must be equal or differ by 1")

    if count['X'] == count['O']:
        current_player = 'X'
    else:
        current_player = 'O' if count['O'] < count['X'] else 'X'

    return board, current_player

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Simple Gomoku Game")
    
    parser.add_argument('--size', type=int, default=19, help='Board size (default: 19)')
    parser.add_argument('--black', type=str, default=None, help='Name of black stone player (X)')
    parser.add_argument('--white', type=str, default=None, help='Name of white stone player (O)')
    parser.add_argument('--board', type=str, help='Path to board file')

    args = parser.parse_args()

    board = None
    current_player = 'X'  # default

    if args.board:
      try:
          board, current_player = load_and_validate_board(args.board)
          print(f"Loaded board from {args.board}")
      except Exception as e:
          print(f"Failed to load board: {e}")
          exit(1)

    print(f"Board size: {args.size}")
    print(f"Black player: {args.black}")
    print(f"White player: {args.white}")

    root = tk.Tk()
    app = GomokuGUI(root, args.black, args.white)

    # if board is passed, update game state
    if board:
        app.game.board = board
        app.game.current_player = current_player
        app.draw_board(board)
        app.update_turn_label()

    root.mainloop()
