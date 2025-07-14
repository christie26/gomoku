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
    
    def get_free_threes_from_move(self, x, y):
        '''
        This function return new free-three arrays from a move.
        '''
        def find_sublist(lst, sub):
          for i in range(len(lst) - len(sub) + 1):
              if lst[i:i+len(sub)] == sub:
                  return i
          return -1
        
        new_free_threes = []
        directions = [(1, -1), (1, 0), (1, 1), (0, 1)]
        free_three_pattern = [[0,1,1,1,0], [0,1,0,1,1,0], [0,1,1,0,1,0]]

        for dx, dy in directions:
            array = self.make_array(x, y, -4, 4, dx, dy)
            for pattern in free_three_pattern:
              find = find_sublist(array, pattern)
              if find != -1:
                  point_array = [(x + dx * i, y + dy * i) for i in range(find - 4, find + len(pattern) - 4)]
                  new_free_threes.append(point_array)
                      
        return new_free_threes
        
    def capture(self, x0, y0):
        """
        A function to detect if a move causes a capture.
        """
        directions = [(1, -1), (1, 0), (1, 1), 
                      (0, -1), (0, 1), 
                      (-1, -1), (-1, 0), (-1, 1)]
        capture_count = 0
        for dx, dy in directions:
            x, y = x0, y0
            array = self.make_array(x0, y0, 0, 3, dx, dy)

            if array == [1, -1, -1, 1]:
                self.board[x + dx][y + dy] = '.'
                self.board[x + dx*2][y + dy*2] = '.'
                self.remove_free_three(x + dx, y + dy, self.opponent_player)
                self.remove_free_three(x + dx*2, y + dy*2, self.opponent_player)
                capture_count += 1
        if capture_count > 0:
            self.capture_count[self.current_player] += capture_count
            return True
        return False
    
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

    def check_draw(self):
        # TODO This should also check if there are no valid moves
        return sum(row.count('.') for row in self.board) == 0

    def count_empty_spots(self):
        return sum(row.count('.') for row in self.board)

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
