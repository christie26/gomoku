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
        self.free_three_list = {"X": [], "O": []}
        self.five_row = {"X": None, "O": None}
        self.win_capture_count = 5
        self.current_move = None, None
        # self.change = []

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

        self.board[x][y] = "."

        if len(new_free_threes) > 1:
            return True
        else:
            self.add_free_threes(new_free_threes, self.current_player)
            return False

    def get_free_threes_from_move(self, x0, y0):
        """
        This function return new free-three arrays from a move.
        """

        def count_free_three(sign, dx, dy):
            my_count = 0
            empty_count = 0
            i = 1
            hole = False
            while True:
                x, y = x0 + dx * i * sign, y0 + dy * i * sign
                if (
                    not self.is_on_board(x, y)
                    or self.board[x][y] == self.opponent_player
                    or empty_count == 2
                ):
                    break
                if self.board[x][y] == self.current_player:
                    if empty_count > 0:
                        hole = True
                    my_count += 1
                else:
                    empty_count += 1
                i += 1
            return my_count, empty_count, hole

        new_free_threes = []
        directions = [(1, -1), (1, 0), (1, 1), (0, 1)]

        for dx, dy in directions:
            plus_my, plus_empty, plus_hole = count_free_three(+1, dx, dy)
            minus_my, minus_empty, minus_hole = count_free_three(-1, dx, dy)

            if plus_my + minus_my >= 2 and plus_empty + minus_empty >= 3:
                if plus_hole and minus_empty == 2:
                    minus_empty = 1
                if minus_hole and plus_empty == 2:
                    plus_empty = 1
                plus_end = plus_empty + plus_my
                minus_end = minus_empty + minus_my
                points = tuple(
                    (x0 + dx * i, y0 + dy * i) for i in range(-minus_end, plus_end + 1)
                )
                new_free_threes.append(points)
        return new_free_threes

    def get_free_threes_from_capture(self, x0, y0):
        """
        This function return new free-three arrays from a capture.
        """

        def count_free_three(sign, dx, dy):
            my_count = 0
            empty_count = 0
            i = 1
            hole = False
            while True:
                x, y = x0 + dx * i * sign, y0 + dy * i * sign
                if (
                    not self.is_on_board(x, y)
                    or self.board[x][y] == self.opponent_player
                    or empty_count == 2
                ):
                    break
                if self.board[x][y] == self.current_player:
                    if empty_count > 0:
                        hole = True
                    my_count += 1
                else:
                    empty_count += 1
                i += 1
            return my_count, empty_count, hole

        new_free_threes = []
        directions = [(1, -1), (1, 0), (1, 1), (0, 1)]

        for dx, dy in directions:
            plus_my, plus_empty, plus_hole = count_free_three(+1, dx, dy)
            minus_my, minus_empty, minus_hole = count_free_three(-1, dx, dy)
            if (plus_my == 3 and plus_empty == 2) or (
                minus_my == 3 and minus_empty == 2
            ):
                if plus_my == 3 and plus_empty == 2:
                    points = tuple(
                        (x0 + dx * i, y0 + dy * i)
                        for i in range(0, plus_my + plus_empty + 1)
                    )
                    new_free_threes.append(points)
                if minus_my == 3 and minus_empty == 2:
                    points = tuple(
                        (x0 + dx * i, y0 + dy * i)
                        for i in range(-(minus_my + minus_empty), 0 + 1)
                    )
                    new_free_threes.append(points)
            elif plus_hole == False and minus_hole == False:
                if plus_my + minus_my == 3 and plus_empty and minus_empty:
                    plus_end = plus_my + 1
                    minus_end = minus_my + 1
                    points = tuple(
                        (x0 + dx * i, y0 + dy * i)
                        for i in range(-(minus_end), plus_end + 1)
                    )
                    new_free_threes.append(points)
        return new_free_threes

    def capture_center(self, x0, y0) -> int:
        """
        A function to detect if a move causes a capture.
        """

        def is_capture(dx, dy):
            for i in range(1, 3):
                x, y = x0 + dx * i, y0 + dy * i
                if self.board[x][y] != self.opponent_player:
                    return False
            x, y = x0 + dx * 3, y0 + dy * 3
            if self.board[x][y] == self.current_player:
                return True
            return False

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
            if is_capture(dx, dy):
                self.apply_capture(x0, y0, dx, dy)
                capture_count += 1

        if capture_count > 0:
            self.capture_count[self.current_player] += capture_count
        return capture_count

    def apply_capture(self, x0, y0, dx, dy):
        """
        A function to apply a capture.
        Remove opponent's stone, remove opponent's free-threes, find my new free-threes.
        """
        new_free_threes = []
        for i in range(1, 3):
            x, y = x0 + dx * i, y0 + dy * i
            self.board[x][y] = "."
            # self.change.append((x, y))
            self.remove_free_three(x, y, self.opponent_player)
        for i in range(1, 3):
            x, y = x0 + dx * i, y0 + dy * i
            new_free_threes.append(self.get_free_threes_from_capture(x, y))

        self.add_free_threes(new_free_threes, self.current_player)

    def check_five_rows_from_move(self, x0, y0):
        def count_five(sign, dx, dy):
            my_count = 0
            i = 1
            while True:
                x, y = x0 + dx * i * sign, y0 + dy * i * sign
                if (
                    not self.is_on_board(x, y)
                    or self.board[x][y] != self.current_player
                ):
                    break
                my_count += 1
                i += 1
            return my_count

        directions = [(1, 0), (0, 1), (1, 1), (1, -1)]

        for dx, dy in directions:
            plus_my = count_five(+1, dx, dy)
            minus_my = count_five(-1, dx, dy)
            if plus_my + minus_my >= 4:
                self.five_row[self.current_player] = tuple(
                    (x0 + dx * i, y0 + dy * i) for i in range(-minus_my, plus_my + 1)
                )

    def add_free_threes(self, new_free_threes, player):
        self.free_three_list[player].extend(
            v for v in new_free_threes if v not in self.free_three_list[player]
        )
        # print(f"After add free three", self.free_three_list[player])

    def remove_free_three(self, x, y, player):
        def filtered_tuple(free_three, x, y):
            if (x, y) not in free_three:
                return free_three
            elif len(free_three) == 7 and (
                (x, y) == free_three[0] or (x, y) == free_three[6]
            ):
                return tuple(i for i in free_three if i != (x, y))
            else:
                return None

        free_threes = self.free_three_list[player]
        self.free_three_list[player] = [
            filtered_tuple(free_three, x, y)
            for free_three in free_threes
            if filtered_tuple(free_three, x, y) is not None
        ]
        # print("After remove free three", self.free_three_list[player])

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
        capture_count = 0
        if result == MoveResult.VALID:
            self.current_move = x, y
            self.board[x][y] = self.current_player
            # self.change.append((x, y))
            self.remove_free_three(x, y, self.opponent_player)
            self.check_five_rows_from_move(x, y)
            capture_count = self.capture_center(x, y)
        return result, capture_count

    def count_empty_spots(self):
        return sum(row.count(".") for row in self.board)

    def get_winner(self):
        x, y = self.current_move

        # 1. five captures
        if self.capture_count[self.current_player] >= self.win_capture_count:
            return self.current_player

        # 2. check if opponent keep five rows
        if x is None or y is None:
            return None
        if self.five_row[self.opponent_player] != None:
            for x, y in self.five_row[self.opponent_player]:
                if self.board[x][y] != self.opponent_player:
                    return None
            return self.opponent_player

        return None

    def check_draw(self):
        # TODO This should also check if there are no valid moves
        return sum(row.count(".") for row in self.board) == 0

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
    #                 if self.get_winner():
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
