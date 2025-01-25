#!/bin/env python3

import chess.pgn
import sys

while True:
    game = chess.pgn.read_game(sys.stdin)

    if game is None:
        break

    r = game.headers.get("Result")
    if r == "1-0":
        r = "1"
    elif r == "0-1":
        r = "0"
    else:
        r = "0.5"

    for node in game.mainline():
        c = node.comment[0]
        if "M" not in c:
            print(f'{node.board().fen()},,{r}')
