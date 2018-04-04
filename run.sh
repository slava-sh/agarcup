#!/bin/bash
export DEBUG_STRATEGY=1
#python3 "$(dirname "$0")/src/main.py"
python3 -m cProfile -o "$(dirname "$0")/profile.txt" "$(dirname "$0")/src/main.py"
