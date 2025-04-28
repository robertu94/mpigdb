#!/usr/bin/env sh

rm -rf build
mkdir -p build

cd build

cmake .. \
    -D CMAKE_C_COMPILER=$(which mpicc) \
    -D CMAKE_CXX_COMPILER=$(which mpic++) \
    -D CMAKE_BUILD_TYPE=Debug

make -j 4

cd ..

rm -rf build-asan
mkdir -p build-asan

cd build-asan

cmake .. \
    -D CMAKE_C_COMPILER=$(which mpicc) \
    -D CMAKE_CXX_COMPILER=$(which mpic++) \
    -D CMAKE_BUILD_TYPE=Debug \
    -D USE_SANITIZER=Address

make -j 4
