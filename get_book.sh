#!/bin/sh

wget https://raw.githubusercontent.com/official-stockfish/books/master/8moves_v3.pgn.zip && unzip 8moves_v3.pgn.zip
curl https://funn.is-a.dev/share/Perfect2023.bin.gz | gzip -cd > Perfect2023.bin
