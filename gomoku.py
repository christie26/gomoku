from enum import Enum
class MoveResult(Enum):
    VALID = 0
    OUT_OF_BOARD = 1
    NOT_EMPTY = 2
    DOUBLE_THREE = 3

class Gomoku:
    def __init__(self, size=19):
        self.size = size
        self.board = [['.' for _ in range(size)] for _ in range(size)]
        self.current_player = 'X'
        self.opponent_player = 'O'
        self.capture_count = {
            'X': 0,
            'O': 0
        }
        self.free_three_count = {
            'X': 0,
            'O': 0,
        }
        self.free_three_array = {
            'X': [],
            'O': []
        }
        self.win_capture_count = 5

    def print_board(self):
        print('  ' + ' '.join(map(lambda x: f"{x:2}", range(self.size))))
        for i, row in enumerate(self.board):
            print(f"{i:2} " + ' '.join(map(lambda x: f"{x:2}", row)))

    def is_on_board(self, x, y):
        return 0 <= x < self.size and 0 <= y < self.size
        
    def is_valid_move(self, x, y):
        if not self.is_on_board(x, y):
            return MoveResult.OUT_OF_BOARD
        elif self.board[x][y] != '.':
            return MoveResult.NOT_EMPTY
        elif self.is_double_three_move(x, y):
            return MoveResult.DOUBLE_THREE
        else:
            return MoveResult.VALID
    
    def is_double_three_move(self, x, y):
      self.board[x][y] = self.current_player
      new_free_threes = self.get_free_threes_from_move(x, y)

      if len(self.free_three_array[self.current_player]) + len(new_free_threes) > 1:
          self.board[x][y] = '.'
          return True
      
      self.free_three_array[self.current_player].extend(new_free_threes)
      print(f"{self.current_player} free-three: {len(self.free_three_array[self.current_player])}")
      print(f"{self.opponent_player} free-three: {len(self.free_three_array[self.opponent_player])}")
      print("\n")
      return False
    
    def get_free_threes_from_move(self, x0, y0):
        '''
        This function increase self.free_three_count if there is a free-double from a move.
        '''
        directions = [(1, -1), (1, 0), (1, 1), (0, 1)]
        new_free_threes = []

        for dx, dy in directions:
            new_free_three = self.get_free_three(x0, y0, dx, dy)
            if len(new_free_three) > 0:
                new_free_threes.append(new_free_three)

        return new_free_threes

    def get_free_three(self, x0, y0, dx, dy):
        x_plus, y_plus = self.get_point_in_4_distance(x0, y0, dx, dy)
        x_minus, y_minus = self.get_point_in_4_distance(x0, y0, -dx, -dy)
        point_count = max(abs(x_minus - x_plus), abs(y_minus - y_plus)) + 1

        array = []
        x, y = x_minus, y_minus
        for _ in range(point_count):
            if self.board[x][y] == self.current_player:
                array.append(1)
            elif self.board[x][y] == self.opponent_player:
                array.append(-1)
            else:
                array.append(0)
            x += dx
            y += dy

        result, i, empty_count = self.is_free_three_in_array(array)
        array_length = empty_count + 4
        free_three = []
        if result:
            for j in range(array_length):
                free_three.append((x_minus + dx * (i - j), y_minus + dy * (i - j)))
        return free_three

    def count_free_threes():
        pass


    def is_free_three_in_array(self, array):
        '''
        Get an array (max length: 9)
        1  : my stone
        0  : empty
        -1 : other's stone

        Return if there is a free-three in the array.
        '''
        my_count = 0
        empty_count = 0
        for i, cell in enumerate(array):
            if cell == 0:
                if my_count == 3 and empty_count < 3:
                    return True, i, empty_count
                if my_count > 0:
                    empty_count += 1
                else:
                    empty_count = 1
            if empty_count > 0 and cell == 1:
                my_count += 1
            if cell == -1:
                empty_count = 0
                my_count = 0
        return False, 0, 0
    
    def capture(self, x0, y0):
        """
        A function to detect if a move causes a capture.
        """
        directions = [(1, -1), (1, 0), (1, 1), 
                      (0, -1), (0, 1), 
                      (-1, -1), (-1, 0), (-1, 1)]
        capture_count = 0
        for dx, dy in directions:
            count = 0
            x = x0 + dx
            y = y0 + dy
            
            while self.is_on_board(x, y) and self.board[x][y] == self.opponent_player:
                x += dx
                y += dy
                count += 1

            if self.is_on_board(x, y):
                continue
            
            if self.board[x][y] == self.current_player and count == 2:
                self.board[x - dx][y - dy] = '.'
                self.board[x - dx*2][y - dy*2] = '.'
                self.remove_free_three(x - dx, y - dy, self.opponent_player)
                self.remove_free_three(x - dx*2, y - dy*2, self.opponent_player)
                capture_count += 1
        if capture_count > 0:
            self.capture_count[self.current_player] += capture_count
            return True
        return False

    def handle_move(self, x, y):
        result = self.is_valid_move(x, y)
        if result == MoveResult.VALID:
            self.board[x][y] = self.current_player
            self.remove_free_three(x, y, self.opponent_player)
            if self.capture(x, y):
                return result, True
        return result, False
    
    def remove_free_three(self, x, y, player):
        free_threes = self.free_three_array[player]
        for free_three in free_threes:
            for point in free_three:
                if (x, y) == point:
                    free_threes.remove(free_three)
                    break

    def get_point_in_4_distance(self, x, y, dx, dy):
        for _ in range(4):
            x += dx
            y += dy
            if not self.is_on_board(x, y):
                return x - dx, y - dy
        return x, y

    def check_winner(self):
        # 1. 5 captures
        if self.capture_count[self.current_player] >= self.win_capture_count:
            return True
        
        # 2. 5 stones in a row
        # TODO don't have to check whole board
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
        self.opponent_player = 'O' if self.opponent_player == 'X' else 'X'

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
