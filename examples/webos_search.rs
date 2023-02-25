use std::net::SocketAddr;

use ssdp::header::{HeaderMut, HeaderRef, Location, Man, MX, ST};
use ssdp::message::{Multicast, SearchRequest, SearchResponse};
use ssdp::FieldMap;

fn main() {
    let mut request = SearchRequest::new();
    request.set(Man);
    request.set(ST::All);
    request.set(MX(5));

    println!("Request: {request:?}");
    // Iterate Over Streaming Responses
    for (response, src) in request.multicast().unwrap() {
        let tv = get_tv_network(response.clone(), src);
        if let Some(tv) = tv {
            println!("Tv: {tv:?}");
        } else {
            println!("response: {response:?}");
        }
    }
}

fn get_tv_network(response: SearchResponse, socket_address: SocketAddr) -> Option<TVNetwork> {
    let lg_id = "dial-multiscreen-org:service:dial:1".to_string();
    let filter = FieldMap::urn(lg_id);
    let filter = ST::Target(filter);

    let regex_mac = regex::Regex::new(r"MAC=(?P<mac_address>.?+);").unwrap();
    let regex_ip = regex::Regex::new(r"http://(?P<ip_address>.?+):").unwrap();

    let st = response.get::<ST>()?;
    let loc = response.get::<Location>()?;
    let wake_up = response.get_raw("WAKEUP")?;
    if *st != filter {
        return None;
    }

    let wake_up_values = std::str::from_utf8(&wake_up[0]).unwrap_or("");

    let mac_address = regex_mac
        .captures(wake_up_values)?
        .name("mac_address")?
        .as_str()
        .to_string();

    let _ip = regex_ip
        .captures(&loc.to_string())?
        .name("ip_address")?
        .as_str()
        .to_string();

    let tv = TVNetwork {
        name: "LG WebOS 1.5".to_string(),
        ip: socket_address.ip().to_string(),
        mac_address,
    };

    Some(tv)
}
#[derive(Debug)]
struct TVNetwork {
    name: String,
    ip: String,
    mac_address: String,
}
