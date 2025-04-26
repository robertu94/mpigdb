#!/usr/bin/env bash

NPROC=$1
MPIGDB="../target/release/mpigdb"
nsecs=5

if [[ -z "$NPROC" ]]; then
    echo "Please specify number of procs e.g., ./benchmark.sh 8"
    exit 1
fi

file="release-${NPROC}.out"

mpirun -n $NPROC /usr/bin/time ./build/heatdist > $file 2>&1
echo "finished $file"

file="asan-${NPROC}.out"

mpirun -n $NPROC /usr/bin/time ./build-asan/heatdist > $file 2>&1
echo "finished $file"

file="valgrind-${NPROC}.out"

mpirun -n $NPROC /usr/bin/time valgrind ./build/heatdist > $file 2>&1
echo "finished $file"

file="mpigdb-asan-${NPROC}.out"

$MPIGDB --mpigdb_dbg_arg -x --mpigdb_dbg_arg debug-scripts/mpigdb-script.sh -np $NPROC -- ./build-asan/heatdist > $file 2>&1
echo "finished $file"

mdb launch -n $NPROC -b gdb -t ./build-asan/heatdist > /dev/null 2>&1 &
sleep ${nsecs}
file="mdb-asan-${NPROC}.out"
mdb attach -h 127.0.1.1 -p 2000 -x debug-scripts/asan.mdb > $file 2>&1
echo "finished $file"

echo "sleeping ${nsecs}s..."
sleep ${nsecs}

mdb launch -n $NPROC -b vgdb -t ./build/heatdist > /dev/null 2>&1 &
sleep ${nsecs}
file="mdb-valgrind-${NPROC}.out"
mdb attach -h 127.0.1.1 -p 2000 -x debug-scripts/valgrind.mdb > $file 2>&1
echo "finished $file"

grep "time:" *"-${NPROC}.out"
