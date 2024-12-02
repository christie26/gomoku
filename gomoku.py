class Gomoku:
    def __init__(self, size=15):
        self.size = size
        self.board = [['.' for _ in range(size)] for _ in range(size)]
        self.current_player = 'X'

    def print_board(self):
        print('  ' + ' '.join(map(str, range(self.size))))
        for i, row in enumerate(self.board):
            print(f"{i:2}" + ' '.join(row))

    def is_valid_move(self, x, y):
        return 0 <= x < self.size and 0 <= y < self.size and self.board[x][y] == '.'

    def make_move(self, x, y):
        if self.is_valid_move(x, y):
            self.board[x][y] = self.current_player
            return True
        return False

    def check_winner(self):
        directions = [(1, 0), (0, 1), (1, 1), (1, -1)]
        for x in range(self.size):
            for y in range(self.size):
                if self.board[x][y] == self.current_player:
                    for dx, dy in directions:
                        if self.is_five_in_a_row(x, y, dx, dy):
                            return True
        return False

    def is_five_in_a_row(self, x, y, dx, dy):
        count = 0
        for i in range(5):
            nx, ny = x + i * dx, y + i * dy
            if 0 <= nx < self.size and 0 <= ny < self.size and self.board[nx][ny] == self.current_player:
                count += 1
            else:
                break
        return count == 5

    def switch_player(self):
        self.current_player = 'O' if self.current_player == 'X' else 'X'

    def play(self):
        print("Welcome to Gomoku!")
        self.print_board()
        while True:
            print(f"Player {self.current_player}'s turn.")
            try:
                x, y = map(int, input("Enter your move (row and column): ").split())
                if self.make_move(x, y):
                    self.print_board()
                    if self.check_winner():
                        print(f"Player {self.current_player} wins!")
                        break
                    self.switch_player()
                else:
                    print("Invalid move. Try again.")
            except ValueError:
                print("Please enter valid numbers separated by a space.")
        print("Game Over")


if __name__ == "__main__":
    game = Gomoku()
    game.play()
