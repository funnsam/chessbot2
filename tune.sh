#!/bin/sh

cd tuner
# (cat ../games*.pgn | ../extract_tune.py) | cargo r -r && cargo clean -p chessbot2 -r && cd .. && ./build.sh $1

# https://bitbucket.org/zurichess/tuner/downloads/tuner.7z
cat ../tune-data/quiet-labeled.epd | sed -e 's/c9 /0 1,,/' -e 's/\"1-0\"/1/' -e 's/\"1\/2-1\/2\"/0.5/' -e 's/\"0-1\"/0/' -e 's/;$//' | \
    cargo r -r && cargo clean -p chessbot2 -r && cd .. && ./build.sh $1
