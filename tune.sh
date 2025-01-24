#!/bin/sh

cd tuner
cat ../self_play* | cargo r -r
