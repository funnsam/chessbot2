#!/bin/sh

cd tuner
cat ../self_play* | cargo r -r && cargo clean -p chessbot2 && cd .. && ./build.sh $1
