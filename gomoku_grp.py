import tkinter as tk
from tkinter import messagebox

class GomokuGame:
    def __init__(self, size=15, cell_size=40):
        self.size = size
        self.cell_size = cell_size
        self.current_player = 'X'
        self.board = [['.' for _ in range(size)] for _ in range(size)]
        self.window = tk.Tk()
        self.window.title("Gomoku")
        self.canvas = tk.Canvas(
            self.window,
            width=size * cell_size,
            height=size * cell_size,
            bg="white"
        )
        self.canvas.pack()
        self.canvas.bind("<Button-1>", self.click_event)
        self.draw_board()

    def draw_board(self):
        """Draw the board grid."""
        for i in range(self.size):
            self.canvas.create_line(
                0, i * self.cell_size,
                self.size * self.cell_size, i * self.cell_size,
                fill="black"
            )
            self.canvas.create_line(
                i * self.cell_size, 0,
                i * self.cell_size, self.size * self.cell_size,
                fill="black"
            )

    def draw_piece(self, x, y):
        """Draw a piece at the given grid position."""
        x1 = x * self.cell_size + self.cell_size // 4
        y1 = y * self.cell_size + self.cell_size // 4
        x2 = (x + 1) * self.cell_size - self.cell_size // 4
        y2 = (y + 1) * self.cell_size - self.cell_size // 4
        color = "black" if self.current_player == 'X' else "white"
        self.canvas.create_oval(x1, y1, x2, y2, fill=color, outline="black")

    def click_event(self, event):
        """Handle mouse click events."""
        x = event.x // self.cell_size
        y = event.y // self.cell_size
        if 0 <= x < self.size and 0 <= y < self.size and self.board[y][x] == '.':
            self.board[y][x] = self.current_player
            self.draw_piece(x, y)
            if self.check_winner(x, y):
                messagebox.showinfo("Game Over", f"Player {self.current_player} wins!")
                self.window.destroy()
            else:
                self.switch_player()

    def switch_player(self):
        """Switch the current player."""
        self.current_player = 'O' if self.current_player == 'X' else 'X'

    def check_winner(self, x, y):
        """Check if the current player has won."""
        directions = [(1, 0), (0, 1), (1, 1), (1, -1)]
        for dx, dy in directions:
            if self.count_consecutive(x, y, dx, dy) + self.count_consecutive(x, y, -dx, -dy) - 1 >= 5:
                return True
        return False

    def count_consecutive(self, x, y, dx, dy):
        """Count consecutive pieces in a given direction."""
        count = 0
        player = self.current_player
        while 0 <= x < self.size and 0 <= y < self.size and self.board[y][x] == player:
            count += 1
            x += dx
            y += dy
        return count

    def run(self):
        """Run the game."""
        self.window.mainloop()

if __name__ == "__main__":
    game = GomokuGame()
    game.run()
