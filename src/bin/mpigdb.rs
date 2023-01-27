use std::fs::OpenOptions;
use std::io::prelude::*;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
#[derive(Debug)]
struct CLIArgs {
    procs: Vec<usize>,
    base_port: usize,
    global_mpi_args: Vec<String>,
    mpi_args: Vec<Vec<String>>,
    prg_args: Vec<Vec<String>>,
    helper: String,
    dry_run: bool,
}

fn write_startup_file(hostports: &[String]) -> anyhow::Result<()> {
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(".startup.gdb")?;
    let hostport = &hostports[0];
    f.write_all(
        format!(
            "
set pagination off
set non-stop on
set sysroot /
set exec-file-mismatch off

python
{}
end
    ",
            include_str!("mpi_gdbhelpers.py")
        )
        .as_ref(),
    )?;
    f.write_all(
        format!(
            "
target extended-remote {hostport}
    "
        )
        .as_ref(),
    )?;

    for (i, host) in hostports.iter().skip(1).enumerate() {
        let inferrior = i + 2;
        f.write_all(
            format!(
                "
add-inferior -no-connection
inferior {inferrior}
target extended-remote {host}
        "
            )
            .as_ref(),
        )?;
    }
    f.sync_all()?;

    Ok(())
}

enum CliState {
    MpiFlags,
    CommandFlags,
    ProcsFlag,
    PortFlag,
    HelperFlag,
    GlobalFlag,
}

fn parse_args() -> anyhow::Result<CLIArgs> {
    let mut helper = "mpigdb_helper".to_string();
    let mut base_port = 8000;
    let mut dry_run = false;
    let mut state = CliState::MpiFlags;

    let mut procs = vec![1];
    let mut mpi_args: Vec<Vec<String>> = vec![Vec::new()];
    let mut prg_args: Vec<Vec<String>> = vec![Vec::new()];
    let mut global_args = Vec::new();

    for arg in std::env::args().skip(1) {
        state = match &state {
            CliState::MpiFlags => match &*arg {
                "-n" | "-np" => CliState::ProcsFlag,
                "--mpigdb_helper" => CliState::HelperFlag,
                "--mpigdb_port" => CliState::PortFlag,
                "--mpigdb_mpi_flag" => CliState::GlobalFlag,
                "--mpigdb_dryrun" => {
                    dry_run = true;
                    CliState::MpiFlags
                }
                "--" => CliState::CommandFlags,
                _ => {
                    mpi_args.last_mut().unwrap().push(arg);
                    CliState::MpiFlags
                }
            },
            CliState::CommandFlags => match &*arg {
                ":" => {
                    prg_args.push(Vec::new());
                    mpi_args.push(Vec::new());
                    procs.push(1);

                    CliState::MpiFlags
                }
                _ => {
                    prg_args.last_mut().unwrap().push(arg);
                    CliState::CommandFlags
                }
            },
            CliState::ProcsFlag => {
                let last = procs.last_mut().unwrap();
                *last = arg.parse::<usize>()?;
                CliState::MpiFlags
            }
            CliState::HelperFlag => {
                helper = arg;
                CliState::MpiFlags
            }
            CliState::PortFlag => {
                base_port = arg.parse()?;
                CliState::MpiFlags
            }
            CliState::GlobalFlag => {
                global_args.push(arg);
                CliState::MpiFlags
            }
        }
    }

    Ok(CLIArgs {
        procs,
        base_port,
        mpi_args,
        prg_args,
        helper,
        dry_run,
        global_mpi_args: global_args,
    })
}

fn main() -> anyhow::Result<()> {
    let args = parse_args()?;
    println!("{args:?}");

    let total_procs = args.procs.iter().sum();
    let mut mpiexec_args: Vec<String> = Vec::new();
    let control_host = hostname::get()?.into_string().unwrap();
    let base_port = args.base_port;
    let control_port = format!("{control_host}:{base_port}");
    println!("listening {control_port}");

    mpiexec_args.extend(args.global_mpi_args);
    let mut i = 0;
    for group in 0..args.procs.len() {
        for _p in 0..args.procs[group] {
            mpiexec_args.extend(args.mpi_args[group].clone());
            mpiexec_args.push(args.helper.clone());
            mpiexec_args.push(control_port.clone());
            mpiexec_args.push((args.base_port + i + 1).to_string());
            mpiexec_args.extend(args.prg_args[group].clone());
            if i < total_procs - 1 {
                mpiexec_args.push(":".to_string())
            }
            i += 1;
        }
    }
    println!("{mpiexec_args:?}");

    if !args.dry_run {
        let (controllistening_send, controllistening_recv) = channel();
        let (hostsalive_send, hostsalive_recv) = channel();
        let hostports = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let th_hostports = std::sync::Arc::clone(&hostports);
        std::thread::spawn(move || {
            let conn_server = std::net::TcpListener::bind(control_port).expect("failed to listen");
            controllistening_send.send(true).expect("failed to wait");
            for stream in conn_server.incoming().take(total_procs) {
                let mut hostport = String::new();
                stream
                    .unwrap()
                    .read_to_string(&mut hostport)
                    .expect("failed to recv");
                th_hostports
                    .lock()
                    .expect("failed to add port")
                    .push(hostport);
            }
            hostsalive_send.send(true).unwrap();
        });

        controllistening_recv.recv()?;
        Command::new("mpiexec")
            .args(mpiexec_args)
            .stdin(Stdio::null())
            .spawn()?;

        //write startup file
        hostsalive_recv.recv()?;
        let hosts = hostports.lock().unwrap();
        write_startup_file(&*hosts)?;

        Command::new("gdb")
            .arg("-x")
            .arg(".startup.gdb")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .exec();
    }

    Ok(())
}
