use std::fs::OpenOptions;
use std::io::prelude::*;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use serde::Serialize;

#[derive(Debug)]
enum DebugFrontend {
    GDB,
    VSCode
}

#[derive(Debug)]
struct CLIArgs {
    procs: Vec<usize>,
    base_port: usize,
    global_mpi_args: Vec<String>,
    mpi_args: Vec<Vec<String>>,
    dbg_args: Vec<String>,
    prg_args: Vec<Vec<String>>,
    gdbserver: String,
    gdb: String,
    frontend: DebugFrontend,
    helper: String,
    dry_run: bool,
    verbose: bool,
}

#[derive(Serialize)]
struct VscodeEnvionment {
    name: String,
    value: String
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct VscodeStartupCommands {
    description: String,
    text: String,
    ignoreFailures: bool,
}


#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[derive(Serialize)]
struct VscodeConfiguration {
    name: String,
    program: String,
    miDebuggerServerAddress: String,
    r#type : String,
    request: String,
    args: Vec<String>,
    stopAtEntry: bool,
    cwd: String,
    environment: Vec<VscodeEnvionment>,
    externalConsole: bool,
    MIMode: String,
     #[serde(skip_serializing_if = "Option::is_none")]
    miDebuggerPath: Option<String>,
     #[serde(skip_serializing_if = "Option::is_none")]
    miDebuggerArgs: Option<String>,
    setupCommands: Vec<VscodeStartupCommands>

}
#[derive(Serialize)]
struct VscodeCompound {
    name: String,
    configurations: Vec<String>
}

#[derive(Serialize)]
struct VscodeLaunchJson {
    version: String,
    compounds: Vec<VscodeCompound>,
    configurations: Vec<VscodeConfiguration>
}

impl VscodeLaunchJson {
    fn new(args: &CLIArgs, hosts: &Vec<String>) -> VscodeLaunchJson {

        let mut configurations_names = Vec::new();
        let mut configurations = Vec::new();
        let mut idx_to_program = std::collections::HashMap::<usize, String>::new();

        let mut i: usize = 0;
        for group in 0..args.procs.len() {
            for _p in 0..args.procs[group] {
                idx_to_program.insert(i,args.prg_args[group].first().unwrap().clone());
                i+=1;
            }
        }

        for (idx, host) in hosts.iter().enumerate() {
            let name = format!("debug rank {}", idx);
            let mut host  = host.clone();
            host.pop();

            configurations.push(VscodeConfiguration {
                name: name.clone(),
                program: idx_to_program[&idx].clone(),
                r#type: "cppdbg".into(),
                miDebuggerServerAddress: host,
                request: "launch".into(),
                args: Vec::new(),
                stopAtEntry: false,
                cwd: "${workspaceRoot}".into(),
                externalConsole: true,
                environment: Vec::new(),
                MIMode: "gdb".into(),
                setupCommands: vec!(
                    VscodeStartupCommands {
                        description: "pretty print outputs".to_string(),
                        text: "-enable-pretty-printing".to_string(),
                        ignoreFailures: true

                    },
                ),
                miDebuggerPath: Some(args.gdb.clone()),
                miDebuggerArgs: None,

            });
            configurations_names.push(name)
        }

        let compounds = VscodeCompound {
            name: "debug all ranks".into(),
            configurations: configurations_names
        };


        VscodeLaunchJson {
            version: "0.2.0".into(),
            compounds: vec!(compounds),
            configurations,
        }
    }
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
    DbgFlag,
    GDBServerPath,
    GDBPath,
    Frontend,
}

const HELPMSG: &str = include_str!("../../README.md");

fn parse_args() -> anyhow::Result<CLIArgs> {
    let mut helper = "mpigdb_helper".to_string();
    let mut base_port = 8000;
    let mut dry_run = false;
    let mut state = CliState::MpiFlags;
    let mut frontend = DebugFrontend::GDB;

    let mut procs = vec![1];
    let mut mpi_args: Vec<Vec<String>> = vec![Vec::new()];
    let mut prg_args: Vec<Vec<String>> = vec![Vec::new()];
    let mut global_args = Vec::new();
    let mut dbg_args = Vec::new();
    let mut verbose = false;
    let mut gdbserver = "gdbserver".to_string();
    let mut gdb = "gdb".to_string();

    for arg in std::env::args().skip(1) {
        state = match &state {
            CliState::MpiFlags => match &*arg {
                "-h" | "--help" => {
                    println!("{}", HELPMSG);
                    std::process::exit(1);
                }
                "-n" | "-np" => CliState::ProcsFlag,
                "--mpigdb_verbose"  => {
                    verbose = true;
                    CliState::MpiFlags
                }
                "--interpreter=mi"  => {
                    dbg_args.push("--interpreter=mi".to_string());
                    CliState::MpiFlags
                }
                s if s.starts_with("--tty=")  => {
                    dbg_args.push(arg);
                    CliState::MpiFlags
                }
                "--mpigdb_frontend" => CliState::Frontend,
                "--mpigdb_dbg_arg" => CliState::DbgFlag,
                "--mpigdb_helper" => CliState::HelperFlag,
                "--mpigdb_gdbserver" => CliState::GDBServerPath,
                "--mpigdb_gdb" => CliState::GDBPath,
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
            CliState::Frontend => {
                match &arg[..] {
                    "gdb" => { frontend = DebugFrontend::GDB; }
                    "vscode" => { frontend = DebugFrontend::VSCode; }
                    &_ => {
                        eprintln!("invalid frontend {}", arg);
                        std::process::exit(1);
                    }
                }
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
            CliState::DbgFlag => {
                dbg_args.push(arg);
                CliState::MpiFlags
            }
            CliState::GDBServerPath => {
                gdbserver = arg.to_string();
                CliState::MpiFlags
            }
            CliState::GDBPath => {
                gdb = arg.to_string();
                CliState::MpiFlags
            }
        }
    }

    Ok(CLIArgs {
        procs,
        base_port,
        mpi_args,
        dbg_args,
        prg_args,
        helper,
        dry_run,
        verbose,
        gdbserver,
        frontend,
        gdb,
        global_mpi_args: global_args,
    })
}

fn main() -> anyhow::Result<()> {
    let args = parse_args()?;
    if args.verbose {
        eprintln!("{args:?}");
    }

    let total_procs = args.procs.iter().sum();
    let mut mpiexec_args: Vec<String> = Vec::new();
    let control_host = hostname::get()?.into_string().unwrap();
    let base_port = args.base_port;
    let control_port = format!("{control_host}:{base_port}");
    if args.verbose {
        eprintln!("listening {control_port}");
    }

    mpiexec_args.extend(args.global_mpi_args.clone());
    let mut i = 0;
    for group in 0..args.procs.len() {
        for _p in 0..args.procs[group] {
            mpiexec_args.push("-np".to_string());
            mpiexec_args.push("1".to_string());
            mpiexec_args.extend(args.mpi_args[group].clone());
            mpiexec_args.push(args.helper.clone());
            mpiexec_args.push(control_port.clone());
            mpiexec_args.push((args.base_port + i + 1).to_string());
            mpiexec_args.push((if args.verbose  {"1"} else {"0"}).to_string());
            mpiexec_args.push(args.gdbserver.clone());
            mpiexec_args.extend(args.prg_args[group].clone());
            if i < total_procs - 1 {
                mpiexec_args.push(":".to_string())
            }
            i += 1;
        }
    }
    if args.verbose {
        eprintln!("{mpiexec_args:?}");
    }

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

        match args.frontend {
            DebugFrontend::VSCode => {
                let vscode_config = VscodeLaunchJson::new(&args, &*hosts);
                match std::fs::create_dir(".vscode") {
                    Ok(_) => {Ok(())},
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists =>  {
                        Ok(())
                    },
                    r => r
                }?;
                let launch_json = std::fs::File::options()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(".vscode/launch.json")?;
                serde_json::to_writer(launch_json, &vscode_config)?;
                eprintln!("wrote launch.json to {}", std::path::Path::new(".vscode/launch.json").canonicalize()?.display())
            }

            DebugFrontend::GDB => {
                write_startup_file(&*hosts)?;
                Command::new(args.gdb)
                    .arg("-x")
                    .arg(".startup.gdb")
                    .args(args.dbg_args)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .exec();
            }
        }
    }

    Ok(())
}
