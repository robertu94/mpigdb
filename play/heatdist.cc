#include <mpi.h>
#include <vector>
#include <cmath>
#include <utility>
#include <iostream>
#include <chrono>
#include <experimental/mdspan>

namespace stdx = std::experimental;
namespace stdc = std::chrono;
constexpr size_t N = 1024;
constexpr size_t T = 1024*30;
using mds = stdx::mdspan<float, stdx::extents<size_t, N,N>>;

void init(int start, int stop, mds s) {
    for (size_t i = start; i < stop; ++i) {
    for (size_t j = 0; j < s.extent(1); ++j) {
        s(i,j) = pow(i-N/2.0, 2) * pow(j-N/2.0, 2);
    }}
}
void diffuse(size_t start, size_t stop,  mds result, mds v) {
    for (size_t i = std::max(size_t{1}, start); i < std::min(stop, N - 1); ++i) {
    for (size_t j = 1; j < result.extent(1) - 1; ++j) {
        result(i,j) =  (
            v(i,   j) + \
            v(i-1, j) + \
            v(i+1, j) + \
            v(i, j-1) + \
            v(i, j+1)) *0.2;
    }}
}
void exchange(size_t start, size_t stop, mds result) {
    int rank, size;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);
    MPI_Comm_size(MPI_COMM_WORLD, &size);
    std::array<float,N> tmp_bot, tmp_top;
    float* bottom = &result(start, 0);
    float* top = &result(stop-1, 0);


    std::copy(top,top+N, tmp_top.data());
    std::copy(bottom,bottom+N, tmp_bot.data());
    MPI_Sendrecv(top, N, MPI_FLOAT, /*dest*/rank?(rank-1):MPI_PROC_NULL, /*sendtag*/0,
                 tmp_bot.data(), N, MPI_FLOAT, /*source*/(rank+1)>=size?MPI_PROC_NULL:(rank+1), /*recvtag*/0,
                 MPI_COMM_WORLD, MPI_STATUS_IGNORE);
    MPI_Sendrecv(bottom, N, MPI_FLOAT, /*dest*/(rank+1)>=size?MPI_PROC_NULL:(rank+1), /*sendtag*/0,
                 tmp_top.data(), N, MPI_FLOAT, /*source*/rank?(rank-1):MPI_PROC_NULL, /*recvtag*/0,
                 MPI_COMM_WORLD, MPI_STATUS_IGNORE);
    std::copy(tmp_top.begin(), tmp_top.end(), top);
    std::copy(tmp_bot.begin(), tmp_bot.end(), bottom);
}

int main(int argc, char *argv[])
{
    MPI_Init(&argc, &argv);    
    int rank, size;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);
    MPI_Comm_size(MPI_COMM_WORLD, &size);

    MPI_Barrier(MPI_COMM_WORLD);
    auto startt = stdc::high_resolution_clock::now();

    std::vector<float> a(N*N), b(N*N);
    mds current(a.data()), prev(b.data());

    const size_t R = N/size;
    const size_t start = rank*R, stop = std::min((rank+1)*R, N);
    init(start,stop, prev);

    for (int i = 0; i < T; ++i) {
        diffuse(start, stop, current, prev);
        exchange(start, stop, current);
        std::swap(current, prev);
    }
    double s = 0;
    size_t M = 0;
    for (size_t i = 0; i < N; ++i) {
        for (int j = 0; j <= N; ++j) {
            s += current(i,j);
            ++M;
        }
    }
    std::cout << s/M << std::endl;

    MPI_Barrier(MPI_COMM_WORLD);
    auto stopt = stdc::high_resolution_clock::now();
    if(rank ==0) {
    std::cout << "time: " << stdc::duration_cast<stdc::duration<double, std::ratio<1>>>(stopt-startt).count() << std::endl;
    }

    MPI_Finalize();
    return 0;
}
