mod dns;

#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;

use clap::Clap;
use async_net::UdpSocket;
use smol::Task;

use std::net::SocketAddr;
use std::sync::Arc;
use std::net::Ipv4Addr;

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

type Result<T> = std::result::Result<T, anyhow::Error>;

fn main() -> Result<()> {
    let options = Options::parse();

    // initialize logger
    let log_level = match options.verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    pretty_env_logger::formatted_timed_builder()
        .filter_level(log_level)
        .init();

    // set ctrl-c handler
    let (s, ctrl_c) = async_channel::bounded(100);
    ctrlc::set_handler(move || {
        let _ = s.try_send(());
    })?;

    // load configured name map
    let names = load_names(&options.name_file_path)?;
    let dns_authority = dns::DnsAuthority::new(names)?;

    smol::run(async {
        Task::spawn(serve_dns(options.addr, dns_authority))
            .detach();
        info!("listening for DNS queries on {}...", options.addr);

        ctrl_c.recv().await?;
        info!("received Ctrl-C, quitting.");
        Ok(())
    })
}

async fn serve_dns(addr: SocketAddr, dns_authority: dns::DnsAuthority) -> Result<()> {
    let socket = UdpSocket::bind(addr).await?;
    debug!("bound socket to {}.", &addr);

    let dns_authority = Arc::new(dns_authority);
    // let auth = &dns_authority;
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

        // have a task per request
        let socket = socket.clone();
        let dns_authority = dns_authority.clone();
        Task::spawn(async move {
            let result = respond(request_bytes, sender_addr, socket, dns_authority).await;
            if let Err(e) = result {
                error!("{}", e);
            }
        }).detach();
    }
}

async fn respond(request_bytes: Vec<u8>, sender_addr: SocketAddr, socket: UdpSocket, dns_authority: Arc<dns::DnsAuthority>) -> Result<()> {
    let response_bytes = dns_authority.answer_query(request_bytes)
        .or_else(|e| Err(anyhow!("error understanding or answering query: {}", e)))?;

    socket.send_to(&response_bytes, sender_addr)
        .await
        .or_else(|e| Err(anyhow!("error sending response: {}", e)))?;

    Ok(())
}

fn load_names(path: &str) -> Result<Vec<(String, Ipv4Addr)>> {
    use std::fs;
    use std::path::PathBuf;

    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(anyhow!("configured names file '{}' does not exist!", path.display()));
    }

    let contents = String::from_utf8(fs::read(&path)?)?;
    let names: Result<Vec<(String, Ipv4Addr)>> = contents.lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with("#"))
        .filter(|l| !l.is_empty())
        .map(|l| {
            let strs: Vec<&str> = l.split("=")
                .map(|s| s.trim())
                .collect();
            if strs.len() != 2 {
                return Err(anyhow!("cannot parse names file: expected lines of the form '<name>=<ipv4>', got '{}'!", l));
            }

            let domain = strs[0].to_owned();
            let addr: Ipv4Addr = strs[1].parse()
                .or_else(|e| Err(anyhow!("cannot parse IPv4 addr from '{}': {}", strs[1], e)))?;
            Ok((domain, addr))
        })
        .collect();

    names
}
