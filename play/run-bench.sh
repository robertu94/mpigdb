#!/usr/bin/env bash

n=$1
for run in {1..5}
do
    rm -f *.log *.out
    bash benchmark.sh $n > "output-run-${run}-n-${n}.dat"
done

