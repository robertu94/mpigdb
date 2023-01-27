# MPIGDB

A wrapper around mpiexec, gdbserver, and gdb that makes debugging MPI programs easier with a moderate number of processes.

## Example Usage

![demo](https://github.com/robertu94/mpigdb/blob/main/docs/demo.gif)

## GDB Extensions and Useful commands

This wrapper defines several GDB extension commands that should help make things easier too.

`mpic` continue, but all processes

`mpict` continue this thread, and switch to the next one that is stopped if there is one

`mpip` print on all or a subset of threads using `-t $tid`

`mpib` break on all or a subset of threads using `-t $tid`

You also should probably know about the following buildin commands

`thread apply all` applies a command to all threads

`continue &` in GDB any command ending in `&` runs a command "in the background" allowing the user to continue to interact with GDB.  In this case continue this thread in the background

`info threads` get a list of threads and their status

`interupt` if the current thread is running stop it.

`thread $tid` switch to thread id

## Building and Installation

Dependencies:

+ `gdb` 12.1 or later with `gdbserver` which is sometimes packaged separately and python support which may be disabled if compiled from source
+ a MPI installation including mpiexec `mpiexec`
+ python 3.8 or later
+ Rust+Cargo 1.65 or later

Earlier versions may work, but are not tested

## Limitations and Known Bugs

For HPC systems with meaningful scale, we should probably integrate with PMIx, but we currently do onot.
