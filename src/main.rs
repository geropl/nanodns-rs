mod dns;

#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;

use clap::Clap;
use async_net::UdpSocket;
use smol::Task;

use std::net::SocketAddr;

#[derive(Clap)]
#[clap(
    name = "nanodns",
    version = "0.1.0",
    author = "Gero Posmyk-Leinemann <gero.posmyk-leinemann@typefox.io>",
    about = "ultra-minimal authorative DNS server"
)]
struct Options {
    /// The local socket address to bind to. ex.: 127.0.0.1:53
    #[clap(parse(try_from_str = parse_socket_addr))]
    addr: SocketAddr,

    /// The path to the name file to use
    #[clap(
        short = "n",
        long = "names",
        default_value = "./names.conf"
    )]
    name_file_path: String,

    /// Controls the log level. ex.: -v,  -vv or -vvv
    #[clap(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: u16,
}

fn parse_socket_addr(addr: &str) -> std::result::Result<SocketAddr, std::net::AddrParseError> {
    addr.parse()
}

type Result = std::result::Result<(), anyhow::Error>;

fn main() -> Result {
    let options = Options::parse();

    let log_level = match options.verbose {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    pretty_env_logger::formatted_timed_builder()
        .filter_level(log_level)
        .init();

    let (s, ctrl_c) = async_channel::bounded(100);
    ctrlc::set_handler(move || {
        let _ = s.try_send(());
    })?;

    smol::run(async {
        Task::spawn(serve_dns(options.addr))
            .detach();
        info!("listening for DNS queries on {}...", options.addr);

        ctrl_c.recv().await?;
        info!("received Ctrl-C, quitting.");
        Ok(())
    })
}


async fn serve_dns(addr: SocketAddr) -> Result {
    let socket = UdpSocket::bind(addr).await?;
    debug!("bound socket to {}.", &addr);

    let mut buf = [0u8; 512];
    loop {
        let (bytes_read, sender_addr) = match socket.recv_from(&mut buf).await {
            Err(e) => {
                error!("error while receiving datagram: {}", e);
                continue;
            },
            Ok(r) => r,
        };
        debug!("received datagram (length: {}, sender: {})", bytes_read, sender_addr);
        let request_bytes = Vec::from(&buf[0..bytes_read]);

        // have a task respond to this request
        let cloned_sender_addr = sender_addr.clone();
        let cloned_socket = socket.clone();
        Task::spawn(async move {
            let result = respond(request_bytes, cloned_sender_addr, cloned_socket).await;
            if let Err(e) = result {
                error!("{}", e);
            }
        }).detach();
    }
}

async fn respond(request_bytes: Vec<u8>, sender_addr: SocketAddr, socket: UdpSocket) -> Result {
    let response_bytes = dns::answer_query(request_bytes)
        .or_else(|e| Err(anyhow!("error understanding or answering query: {}", e)))?;

    socket.send_to(&response_bytes, sender_addr)
        .await
        .or_else(|e| Err(anyhow!("error sending response: {}", e)))?;

    Ok(())
}

