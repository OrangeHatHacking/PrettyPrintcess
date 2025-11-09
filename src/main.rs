use cidr::Ipv4Cidr;
use get_if_addrs::{IfAddr, get_if_addrs};
use local_ip_address::local_ip;
use std::net::{IpAddr, Ipv4Addr};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // get local IP to match to interface
    let machine_ip = match local_ip()? {
        IpAddr::V4(ipv4) => ipv4,
        IpAddr::V6(_) => return Err("IPv6 not supported".into()),
    };

    // get interface
    // And because I know I'll forget, that weird |thing| in the find iterator function is
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
        println!("{}", network_base);
        let cidr = Ipv4Cidr::new(network_base, prefix_len)?;

        println!(
            "Interface: {}\nIP: {}\nNetmask: {}\nNetwork: {}/{}\n",
            iface.name,
            ip,
            netmask,
            cidr.first_address(),
            prefix_len
        );
    }
    Ok(())
}
