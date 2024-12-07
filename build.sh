#!/bin/sh

cd chessbot2-uci && cargo b -r && cd .. && cp target/release/chessbot2-uci "engines/$1"
