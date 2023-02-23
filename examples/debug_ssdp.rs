extern crate log;
extern crate ssdp;

use log::Log;

use ssdp::header::{HeaderMut, Man, MX, ST};
use ssdp::message::{Multicast, SearchRequest};

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn main() {
    log::set_max_level(log::LevelFilter::Debug);
    log::set_logger(&SimpleLogger).unwrap();

    // Create Our Search Request
    let mut request = SearchRequest::new();

    // Set Our Desired Headers (Not Verified By The Library)
    request.set(Man);
    request.set(MX(5));
    request.set(ST::All);

    // Collect Our Responses
    let _responses = request.multicast().unwrap().into_iter().collect::<Vec<_>>();
}
