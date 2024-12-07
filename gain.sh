#!/bin/sh

fastchess -engine "cmd=engines/$1" name=Improved -engine "cmd=engines/$2" name=Base -each tc=8+0.08 -rounds 10 -repeat -concurrency 4 -recover -openings file=8moves_v3.pgn format=pgn order=random -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05
