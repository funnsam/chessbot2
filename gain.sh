#!/bin/sh

fastchess \
    -engine "cmd=engines/$1" name="$1" \
    -engine "cmd=engines/$2" name="$2" \
    -each tc=8+0.08 -rounds 10 -repeat -concurrency 3 -recover \
    -openings file=8moves_v3.pgn format=pgn order=random \
    -pgnout file=games.pgn seldepth=true \
    -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05
