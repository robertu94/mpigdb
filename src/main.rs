use std::fs::OpenOptions;
use std::io::prelude::*;
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
#[derive(Debug)]
struct CLIArgs {
    np: usize,
    base_port: usize,
    mpi_args: Vec<String>,
    prg_args: Vec<String>
}

fn write_startup_file(args: &CLIArgs) -> anyhow::Result<()> {
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(".startup.gdb")?;
    f.write_all(format!("
set pagination off
set non-stop on
set sysroot /
set exec-file-mismatch off

python
{}
end
    ", include_str!("mpi_gdbhelpers.py")).as_ref())?;
    let base_port = args.base_port;
    f.write_all(format!("
target extended-remote localhost:{base_port}
    ").as_ref())?;

    for i in 1..args.np {
        let port = base_port + i;
        let inferrior = i+1;
        f.write_all(format!("
add-inferior -no-connection
inferior {inferrior}
target extended-remote localhost:{port}
        ").as_ref())?;
    }
    f.sync_all()?;

    Ok(())
}

fn parse_args() -> anyhow::Result<CLIArgs> {
    let mut np = num_cpus::get();
    let mut base_port = 8000;
    let mut is_np = false;
    let mut is_port = false;
    let mut after_dash = false;
    let mut mpi_args = Vec::new();
    let mut prg_args = Vec::new();
    for arg in std::env::args().skip(1) {
        if !after_dash {
            if is_np {
                np = arg.parse()?;
                is_np = false;
            } else if is_port {
                base_port = arg.parse()?;
                is_port = false;
            } else {
                if arg == "--np" || arg == "-np" || arg == "-n" {
                    is_np = true;
                } else if arg == "-p" {
                    is_port = true;
                }  else if arg == "--" {
                    after_dash = true;
                } else {
                    mpi_args.push(arg.clone())
                }
            }
        } else {
            prg_args.push(arg.clone())
        }
    }

    Ok(CLIArgs {
        np,
        base_port,
        mpi_args,
        prg_args 
    })
}

fn main() -> anyhow::Result<()> {
    let args = parse_args()?;
    println!("{args:?}");

    let mut mpiexec_args: Vec<String> = Vec::new();
    mpiexec_args.extend(args.mpi_args.clone());
    for i in 0..args.np {
        mpiexec_args.push("gdbserver".to_string());
        mpiexec_args.push("--once".to_string());
        mpiexec_args.push(format!("localhost:{}", args.base_port + i).to_string());
        mpiexec_args.extend(args.prg_args.clone());
        if i < args.np - 1 {
            mpiexec_args.push(":".to_string())
        }
    }
    println!("{mpiexec_args:?}");
    Command::new("mpiexec")
        .args(mpiexec_args)
        .stdin(Stdio::null())
        .spawn()?;

    //write startup file
    write_startup_file(&args)?;

    std::thread::sleep(std::time::Duration::from_secs(1));
    
    Command::new("gdb")
        .arg("-x")
        .arg(".startup.gdb")
        .arg(args.prg_args[0].clone())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .exec();

    Ok(())
}
