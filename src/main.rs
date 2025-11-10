use cidr::Ipv4Cidr;
use futures::stream::{self, StreamExt};
use get_if_addrs::{IfAddr, get_if_addrs};
use local_ip_address::local_ip;
use rand::Rng;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

const ORDERED_PORTS: [u16; 5] = [9100, 631, 515, 1883, 8883];

const CONCURRENCY: usize = 4; // amt of concurrent request threads
const MIN_JITTER_MS: u64 = 50;
const MAX_JITTER_MS: u64 = 400;
const CONNECT_TIMEOUT_SECS: u64 = 2;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let all_ips: Vec<Ipv4Addr> = get_ip_list()?; // ? is used to unwrap the Result object
    for host in &all_ips {
        println!("{}", host);
    }

    let online_printers: Vec<Option<(u16, Ipv4Addr)>> = stream::iter(all_ips)
        .map(|ip| async move { check_ports(ip).await })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await; // these last 2 (collect and await) handle all the async thread mess

    let mut printers_map: HashMap<u16, Vec<Ipv4Addr>> = HashMap::new();
    for (port, ip) in online_printers.into_iter().flatten() {
        printers_map.entry(port).or_default().push(ip); // 
        // or_default will create value entry if one doesn't exist for key (port)
    }

    Ok(())
}

async fn check_ports(ip: Ipv4Addr) -> Option<(u16, Ipv4Addr)> {
    let mut rng = rand::rng();
    let initial_jitter = rng.random_range(MIN_JITTER_MS..=MAX_JITTER_MS);
    sleep(Duration::from_millis(initial_jitter)).await;

    for &port in &ORDERED_PORTS {
        let per_port_jitter = rng.random_range(0..=100);
        sleep(Duration::from_millis(per_port_jitter)).await;

        let sock = SocketAddrV4::new(ip, port);
        if try_connect(sock).await {
            return Some((port, ip));
        }
    }
    None
}

async fn try_connect(sock: SocketAddrV4) -> bool {
    let addr = SocketAddr::V4(sock);
    match timeout(
        Duration::from_secs(CONNECT_TIMEOUT_SECS),
        TcpStream::connect(addr),
    )
    .await
    {
        Ok(Ok(_stream)) => true,
        _ => false,
    }
}

fn get_ip_list() -> Result<Vec<Ipv4Addr>, Box<dyn std::error::Error>> {
    // get local IP to match to interface
    let machine_ip = match local_ip()? {
        IpAddr::V4(ipv4) => ipv4,
        IpAddr::V6(_) => return Err("IPv6 not supported".into()),
    };

    // get interface
    // And because I know I'll forget, that weird |thing| in the find iterator function
    // It's a closure (an anonymous function) and it takes iface as an argument
    // If you've forgotten what it is look it up
    let iface = get_if_addrs()?
        .into_iter()
        // matches!(expr, PATTERN if GUARD) expands to true if expr matches PATTERN and the optional if guard is true
        .find(|iface| matches!(&iface.addr, IfAddr::V4(v4) if v4.ip == machine_ip))
        .ok_or("Couldn't find current network interface")?;

    if let IfAddr::V4(iface_info) = iface.addr {
        let ip = iface_info.ip;
        let netmask = iface_info.netmask;
        let prefix_len = u32::from(netmask).count_ones() as u8;

        // use bitwise AND between local IP and netmask to get base IP
        let network_base = Ipv4Addr::from(u32::from(ip) & u32::from(netmask));
        let cidr_object = Ipv4Cidr::new(network_base, prefix_len)?;

        println!(
            "Interface: {}\nIP: {}\nNetmask: {}\nNetwork: {}/{}\n",
            iface.name,
            ip,
            netmask,
            cidr_object.first_address(),
            prefix_len
        );

        let mut hosts: Vec<_> = cidr_object.iter().map(|host| host.address()).collect();

        // remove broadcast & network
        if hosts.len() > 2 {
            hosts.remove(0);
            hosts.pop();
        }

        return Ok(hosts);
    }

    Err("No interface for IPv4 found".into())
}
