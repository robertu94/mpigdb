set confirm off
set breakpoint pending on
mpib __asan::ReportGenericError
commands
bt
quit
end
mpic
