#!/bin/sh

cd tuner
(cat ../games*.pgn | ../extract_tune.py) | cargo r -r && cargo clean -p chessbot2 -r && cd .. && ./build.sh $1
