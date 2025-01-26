#!/bin/sh

if uname -r | grep -q "PRoot-Distro"; then
    termux-wake-lock &
fi

fastchess \
    -engine "cmd=engines/$2" name="Dysprosium" args="\"setoption name Hash value 16\"" \
    -engine "cmd=engines/stash-14" name="Stash" \
    -each tc=8+0.08 -rounds $1 -repeat -concurrency 4 --force-concurrency -recover -ratinginterval 1 \
    -openings file=UHO_Lichess_4852_v1.epd format=epd order=random \
    -pgnout file=games-vs-stash.pgn nodes=true nps=true

if uname -r | grep -q "PRoot-Distro"; then
    termux-wake-unlock &
fi
