set confirm off
set breakpoint pending on
mpib __asan::ReportGenericError
commands
frame
quit
end
mpic
