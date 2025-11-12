mod network_scanner;
mod printer;

use std::collections::HashMap;
use std::net::Ipv4Addr;

use network_scanner::get_online_printers;
use printer::full_steam_ahead;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let printers_map: HashMap<u16, Vec<Ipv4Addr>> = get_online_printers().await?;

    full_steam_ahead(printers_map).await?;

    // iterate through map per port and start blastin'

    Ok(())
}
