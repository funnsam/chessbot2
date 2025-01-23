#!/bin/sh

echo "Engine $2: last modified $(($(date +%s) - $(date -r engines/$2 +%s)))s ago"
echo "Engine $3: last modified $(($(date +%s) - $(date -r engines/$3 +%s)))s ago"

fastchess \
    -engine "cmd=engines/$2" name="$2" \
    -engine "cmd=engines/$3" name="$3" \
    -each tc=40/15 -rounds $1 -repeat -concurrency 4 --force-concurrency -recover \
    -openings file=UHO_Lichess_4852_v1.epd format=epd order=random \
    -pgnout file=games.pgn nodes=true nps=true \
    -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05
