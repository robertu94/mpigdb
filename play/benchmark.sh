#!/usr/bin/env bash

source ~/comp-env.sh

NPROC=$1
nsecs=5

if [[ -z "$NPROC" ]]; then
    echo "Please specify number of procs e.g., ./benchmark.sh 8"
    exit 1
fi

file="release-${NPROC}.out"

mpirun -n $NPROC ./build/heatdist > $file 2>&1
echo "finished $file"

file="asan-${NPROC}.out"

mpirun -n $NPROC ./build-asan/heatdist > $file 2>&1
echo "finished $file"

file="valgrind-${NPROC}.out"

mpirun -n $NPROC valgrind ./build/heatdist > $file 2>&1
echo "finished $file"

file="mpigdb-asan-${NPROC}.out"

mpigdb --mpigdb_dbg_arg -x --mpigdb_dbg_arg debug-scripts/mpigdb-script.sh -np $NPROC -- ./build-asan/heatdist > $file 2>&1
echo "finished $file"

grep "time:" *"-${NPROC}.out"
