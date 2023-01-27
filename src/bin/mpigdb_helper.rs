use std::io::prelude::*;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

pub fn main() -> Result<(), anyhow::Error> {
    let host = hostname::get()?.into_string().unwrap();
    let control_addr = &std::env::args().nth(1).unwrap();
    let port = &std::env::args().nth(2).unwrap();
    let child_args: Vec<String> = std::env::args().skip(3).collect();

    //send back the connection string
    {
        println!("connecting {}", control_addr);
        let mut control = std::net::TcpStream::connect(control_addr)?;
        control.write_all(format!("{host}:{port}\n").as_ref())?;
        control.shutdown(std::net::Shutdown::Both)?
    }

    println!("child {:?}", child_args);
    Command::new("gdbserver")
        .arg("--once")
        .arg(format!("{host}:{port}"))
        .args(child_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .exec();

    Ok(())
}
