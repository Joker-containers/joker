pub mod errors;
pub mod container;
pub mod daemon;


use std::io::{Write};
use clap::{arg, Command};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::str::FromStr;
use crate::daemon::{Daemon, get_config, write_config};
use crate::errors::AbsentHashMapKeyError;

/// The function to get the help message.
pub fn cli() -> Command {
    Command::new("joker")
        .arg_required_else_help(true)
        .about("A cli component of the joker project.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("add")
                .about("Add a new daemon with custom ip and port.")
                .arg(arg!(<DAEMON_NAME> "The name of the daemon."))
                .arg_required_else_help(true)
                .arg(arg!(-i --ip <IP_ADDRESS> "The ip-address of the host."))
                .arg_required_else_help(true)
                .arg(arg!(-p --port <PORT> "The port of the host."))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("checkout")
                .about("Switch to a daemon.")
                .arg(arg!(<DAEMON_NAME> "The name of the daemon to checkout."))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("run")
                .about("Run specified containers on a current daemon.")
                .arg_required_else_help(true)
                .arg(arg!(<CONTAINER_NAME> ... "Stuff to add"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("trace")
                .about("Traces the events on the daemon. Uses stdout by default.")
        )
        .subcommand(
            Command::new("logs")
                .about("Gets the output of the specified container.")
                .arg(arg!(<CONTAINER_NAME> "The name of the container to get logs from. \
                Uses stdout by default"))
                .arg_required_else_help(true),
        )
}

/// Entry function which executes cli commands.
/// It parses the command and its arguments and then calls a
/// corresponding Rust function.
pub fn execute(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let matches = command.clone().get_matches();
    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let daemon_name = sub_matches.get_one::<String>("DAEMON_NAME").expect("Daemon name is required, but not provided.");
            let ip_addr = sub_matches.get_one::<String>("ip").expect("IP address is required, but not provided.");
            let port = sub_matches.get_one::<String>("port").expect("Port number is required, but not provided.");

            match add_daemon(daemon_name, ip_addr, port) {
                Ok(_) => {
                    Ok(())
                }
                Err(err) => {
                    println!("Error while adding daemon: {}", err);
                    Err(err)
                }
            }
        }
        Some(("checkout", sub_matches)) => {
            let daemon_name = sub_matches.get_one::<String>("DAEMON_NAME").expect("required");

            checkout_daemon(daemon_name)
        }
        Some(("run", sub_matches)) => {
            let containers = sub_matches
                .get_many::<String>("CONTAINER_NAME")
                .into_iter()
                .flatten()
                .map(|x| x.as_str())
                .collect::<Vec<_>>();

            run_containers(&containers)
        }
        Some(("trace", _)) => {
            daemon_trace()
        }
        Some(("logs", sub_matches)) => {
            todo!()
        }
        _ => {
            println!("Error: no such subcommand.");
            show_help_message(command)
        },
    }
}

fn add_daemon(daemon_name: &str, ip_addr: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = daemon::get_config()?;

    let socket_addr = SocketAddr::new(IpAddr::from_str(ip_addr)?, port.parse()?);

    config.daemons.entry(daemon_name.to_owned()).or_insert(socket_addr);

    println!(
        "Added daemon {} at ip {} and port {}.",
        daemon_name,
        ip_addr,
        port,
    );

    write_config(&config)?;

    Ok(())
}

fn checkout_daemon(name: &str) -> Result<(), Box<dyn std::error::Error>> {

    let mut config = daemon::get_config()?;

    match config.daemons.get(name) {
        None => {
            println!(
                "Error while switching to daemon {}: no such daemon.",
                name,
            );

            Err(Box::new(AbsentHashMapKeyError))
        }
        Some(&socket_address) => {
            let name = name.to_owned();

            println!(
                "Switching to daemon {}.",
                name,
            );

            config.current_daemon = Daemon {name, socket_address};

            write_config(&config)?;

            Ok(())
        }
    }
}

fn run_containers(containers: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config()?;

    let mut tcp_stream = TcpStream::connect(config.current_daemon.socket_address)?;

    for &container_path in containers {

        let binary_name = container_path.split('/').last()
            .ok_or("Error: bad file path.")?.as_bytes().to_owned();
        let binary = std::fs::read(container_path)?;
        let binary_config = std::fs::read(format!("{}.joker", container_path))?;

        // Send the size of binary name and binary name itself
        tcp_stream.write_all(&(binary_name.len() as u64).to_le_bytes())?;
        tcp_stream.write_all(&binary_name)?;

        // Send the size of the binary and the binary itself
        tcp_stream.write_all(&(binary.len() as u64).to_le_bytes())?;
        tcp_stream.write_all(&binary)?;

        // Send the size of binary config and binary config itself
        tcp_stream.write_all(&(binary_config.len() as u64).to_le_bytes())?;
        tcp_stream.write_all(&binary_config)?;
    }

    println!(
        "Running containers {} at daemon {}.",
        containers.join(", "),
        "current daemon".to_owned(),
    );

    Ok(())
}

fn daemon_trace() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn get_logs(containers: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn show_help_message(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", command.render_help());
    Ok(())
}
