#include <iostream>
#include <mpi.h>
#include <vector>

void foo(std::vector<int> rank) {
    std::cout << "hi " << rank.front() << std::endl;
    if(rank.at(0)) {
        throw std::runtime_error("test");
    }
}

int main(int argc, char *argv[])
{
    MPI_Init(&argc, &argv);
    int rank;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);
    foo({rank});
    MPI_Finalize();
    return 0;
}
