#!/bin/sh

fastchess \
    -engine "cmd=engines/$1" name="$1" \
    -engine "cmd=engines/$2" name="$2" \
    -each tc=8+0.08 -rounds 10 -repeat -concurrency 4 -recover \
    -openings file=8moves_v3.pgn format=pgn order=random \
    -sprt elo0=-10 elo1=0 alpha=0.05 beta=0.05
