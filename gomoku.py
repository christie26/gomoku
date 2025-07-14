from enum import Enum


class MoveResult(Enum):
    VALID = 0
    OUT_OF_BOARD = 1
    NOT_EMPTY = 2
    DOUBLE_THREE = 3


def find_sublist(lst, sub):
    for i in range(len(lst) - len(sub) + 1):
        if lst[i : i + len(sub)] == sub:
            return i
    return -1


class Gomoku:
    def __init__(self, size=19):
        self.size = size
        self.board = [["." for _ in range(size)] for _ in range(size)]
        self.current_player = "X"
        self.opponent_player = "O"
        self.capture_count = {"X": 0, "O": 0}
        self.free_three_array = {"X": [], "O": []}
        self.five_row = {"X": None, "O": None}
        self.win_capture_count = 5

    def print_board(self):
        print("  " + " ".join(map(lambda x: f"{x:2}", range(self.size))))
        for i, row in enumerate(self.board):
            print(f"{i:2} " + " ".join(map(lambda x: f"{x:2}", row)))

    def is_on_board(self, x, y):
        return 0 <= x < self.size and 0 <= y < self.size

    def is_valid_move(self, x, y):
        if not self.is_on_board(x, y):
            return MoveResult.OUT_OF_BOARD
        elif self.board[x][y] != ".":
            return MoveResult.NOT_EMPTY
        elif self.is_double_three_move(x, y):
            return MoveResult.DOUBLE_THREE
        else:
            return MoveResult.VALID

    def is_double_three_move(self, x, y):
        self.board[x][y] = self.current_player
        new_free_threes = self.get_free_threes_from_move(x, y)

        # if len(self.free_three_array[self.current_player]) + len(new_free_threes) > 1:
        if len(new_free_threes) > 1:
            self.board[x][y] = "."
            return True
        else:
            self.free_three_array[self.current_player].extend(new_free_threes)
            return False

    def get_free_threes_from_move(self, x, y):
        """
        This function return new free-three arrays from a move.
        """

        new_free_threes = []
        directions = [(1, -1), (1, 0), (1, 1), (0, 1)]
        free_three_pattern = [[0, 1, 1, 1, 0], [0, 1, 0, 1, 1, 0], [0, 1, 1, 0, 1, 0]]

        for dx, dy in directions:
            array = self.make_array(x, y, -4, 4, dx, dy)
            for pattern in free_three_pattern:
                find = find_sublist(array, pattern)
                if find != -1:
                    point_array = [
                        (x + dx * i, y + dy * i)
                        for i in range(find - 4, find + len(pattern) - 4)
                    ]
                    new_free_threes.append(point_array)

        return new_free_threes

    def get_free_threes_from_capture(self, x, y):
        """
        This function return new free-three arrays from a capture.
        """

        new_free_threes = []
        directions = [(1, -1), (1, 0), (1, 1), (0, 1)]
        free_three_pattern = [[0, 1, 1, 1, 0], [0, 1, 0, 1, 1, 0], [0, 1, 1, 0, 1, 0]]

        for dx, dy in directions:
            array = self.make_array(x, y, -4, 4, dx, dy)
            for pattern in free_three_pattern:
                find = find_sublist(array, pattern)
                if find != -1:
                    point_array = [
                        (x + dx * i, y + dy * i)
                        for i in range(find - 4, find + len(pattern) - 4)
                    ]
                    new_free_threes.append(point_array)

        return new_free_threes

    def capture_center(self, x, y):
        """
        A function to detect if a move causes a capture.
        """
        directions = [
            (1, -1),
            (1, 0),
            (1, 1),
            (0, -1),
            (0, 1),
            (-1, -1),
            (-1, 0),
            (-1, 1),
        ]
        capture_count = 0
        for dx, dy in directions:
            array = self.make_array(x, y, 0, 3, dx, dy)
            if array == [1, -1, -1, 1]:
                self.apply_capture(x, y, dx, dy)
                capture_count += 1

        if capture_count > 0:
            self.capture_count[self.current_player] += capture_count
            return True
        return False

    def apply_capture(self, x, y, dx, dy):
        """
        A function to apply a capture.
        Remove opponent's stone, remove opponent's free-threes, find my new free-threes.
        """
        new_free_threes = []
        for i in range(2):
            new_x, new_y = x + dx * i, y + dy * i
            self.board[new_x][new_y] = "."
            self.remove_free_three(new_x, new_y, self.opponent_player)
            new_free_threes.append(self.get_free_threes_from_capture(new_x, new_y))

        self.free_three_array[self.current_player].extend(list(set(new_free_threes)))

    def make_array(self, x, y, start_index, end_index, dx, dy):
        array = []

        for i in range(start_index, end_index + 1):
            if not self.is_on_board(x + dx * i, y + dy * i):
                array.append(-2)
                continue
            cell = self.board[x + dx * i][y + dy * i]
            if cell == self.current_player:
                array.append(1)
            elif cell == self.opponent_player:
                array.append(-1)
            else:
                array.append(0)
        return array

    def handle_move(self, x, y):
        result = self.is_valid_move(x, y)
        if result == MoveResult.VALID:
            self.board[x][y] = self.current_player
            self.remove_free_three(x, y, self.opponent_player)
            self.check_five_rows_from_move(x, y)
            if self.capture_center(x, y):
                return result, True
        return result, False

    def remove_free_three(self, x, y, player):
        free_threes = self.free_three_array[player]
        for free_three in free_threes:
            for point in free_three:
                if (x, y) == point:
                    free_threes.remove(free_three)
                    break

    def count_empty_spots(self):
        return sum(row.count('.') for row in self.board)

    def check_winner(self, x, y):
        # 1. five captures
        if self.capture_count[self.current_player] >= self.win_capture_count:
            return self.current_player

        # 2. check if opponent keep five rows
        if self.five_row[self.opponent_player] != None:
            for x, y in self.five_row[self.opponent_player]:
                if self.board[x][y] != self.opponent_player:
                    return None
            return self.opponent_player

        return None

    def check_draw(self):
        # TODO This should also check if there are no valid moves
        return sum(row.count(".") for row in self.board) == 0

    def check_five_rows_from_move(self, x, y):
        directions = [(1, 0), (0, 1), (1, 1), (1, -1)]
        win_pattern = [1, 1, 1, 1, 1]

        for dx, dy in directions:
            array = self.make_array(x, y, -4, 4, dx, dy)
            find = find_sublist(array, win_pattern)
            if find != -1:
                self.five_row[self.current_player] = [
                    (x + dx * i, y + dy * i)
                    for i in range(find - 4, find + len(win_pattern) - 4)
                ]

    def switch_player(self):
        self.current_player = "O" if self.current_player == "X" else "X"
        self.opponent_player = "O" if self.opponent_player == "X" else "X"

    # def play(self):
    #     print("Welcome to Gomoku!")
    #     self.print_board()
    #     while True:
    #         print(f"Player {self.current_player}'s turn. (opponent {self.opponent_player})")
    #         try:
    #             x, y = map(int, input("Enter your move (row and column): ").split())
    #             result = self.handle_move(x, y)
    #             if result == MoveResult.VALID:
    #                 self.print_board()
    #                 if self.check_winner():
    #                     print(f"Player {self.current_player} wins!")
    #                     break
    #                 self.switch_player()
    #             else:
    #                 print("Invalid move:", result.name.replace("_", " ").title())
    #         except ValueError:
    #             print("Please enter valid numbers separated by a space.")
    #     print("Game Over")


if __name__ == "__main__":
    game = Gomoku()
    game.play()
