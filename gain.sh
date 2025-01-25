#!/bin/sh

echo "Engine $2: last modified $(($(date +%s) - $(date -r engines/$2 +%s)))s ago"
echo "Engine $3: last modified $(($(date +%s) - $(date -r engines/$3 +%s)))s ago"

if uname -r | grep -q "PRoot-Distro"; then
    termux-wake-lock &
fi

fastchess \
    -engine "cmd=engines/$2" name="$2" \
    -engine "cmd=engines/$3" name="$3" \
    -each tc=8+0.08 -rounds $1 -repeat -concurrency 4 --force-concurrency -recover -ratinginterval 1 \
    -openings file=8moves_v3.pgn format=pgn order=random \
    -pgnout file=games-$2-vs-$3.pgn nodes=true nps=true \
    -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05

if uname -r | grep -q "PRoot-Distro"; then
    termux-wake-unlock &
fi
