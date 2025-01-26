#!/bin/sh

cd dysprosium-uci && cargo b -r && cd .. && cp target/release/dysprosium-uci "engines/$1"
