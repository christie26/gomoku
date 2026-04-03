NOTE
DONE - scale correctly
DONE - before start game click shouldn't work
DONE - debug tool is off but he can see it
DONE - initialy debug tool didn't work

- disable hint before start 
- debug and hint show different thing 
- debug tool background color

TODO
- 5 row ui

QUESTION
- hint is a button and only show red circle which is the best move in that situation
- hint button goes under undo button (not per player)
- debug is checkbox is not per player 
- when we turn on debug tool, we should show the possible moves 
- debug tool always show the next move
- debug mode only make sense for player


- when game is done, should we disable undo button?

User status
DONE - show player name and if it's AI player or human player
DONE - make timer
DONE - on the side, show how many 'capture' they made

Setting
DONE - user can choose ruleset -> will be implemented later
DONE - disable play_mode, ruleset once game starts
DONE - can toggle debug print on/off
DONE - make start, undo, redo button prettier

redo/undo
DONE - update red point properly
DONE - in case of redo, undo don't trigger timer 
DONE - bind with arrow 

Game board
DONE - add hover when player put mouse on the board
DONE - draw small black circle to make grid prettier
DONE - show position of last move 
DONE - handle invalid move
DONE - before start game, make board gray

Player pannel
DONE - make it seperate file for visibility
DONE - move create_player on click

FIX 
DONE - hover is broken
DONE - activate "start game" button when game is done
DONE - stop timer when game is done
DONE - clean board when new game is started
DONE - reset timer in new game
DONE - update capture live
DONE - capture board ui
DONE - hover is broken