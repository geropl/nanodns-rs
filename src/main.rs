#[macro_use] extern crate log;

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
        info!("started listening for DNS queries on {}...", options.addr);

        ctrl_c.recv().await?;
        info!("received Ctrl-C, quitting.");
        Ok(())
    })
}


async fn serve_dns(addr: SocketAddr) -> Result {
    let socket = UdpSocket::bind(addr).await?;

    let mut buf = [0u8; 512];
    loop {
        info!("recv_from...");
        let (bytes_read, sender_addr) = match socket.recv_from(&mut buf).await {
            Err(e) => {
                error!("error while receiving datagram: {}", e);
                continue;
            },
            Ok(r) => r,
        };
        debug!("received datagram (length: {}, sender: {})", bytes_read, sender_addr);
        let content = Vec::from(&buf[0..bytes_read]);

        Task::spawn(answer_dns_query(content, sender_addr.clone()))
            .detach();
    }
}

async fn answer_dns_query(raw_content: Vec<u8>, sender_addr: SocketAddr) -> Result {
    println!("Content: {:?}", raw_content);
    Ok(())
}
