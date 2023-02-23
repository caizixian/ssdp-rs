use std::fmt::{Formatter, Result};

use hyper::error::{self, Error};
use hyper::header::{Header, HeaderFormat};

use crate::{SSDPErrorKind, SSDPResult};

const MX_HEADER_NAME: &str = "MX";

/// Minimum wait time specified in the `UPnP` 1.0 standard.
pub const MX_HEADER_MIN: u8 = 1;

/// Maximum wait time specified in the `UPnP` 1.0 standard.
pub const MX_HEADER_MAX: u8 = 120;

/// Represents a header used to specify the maximum time that devices should wait
/// before sending a response.
///
/// Should only be increased as the number of devices expected to respond
/// increases, not because of latency or propagation delay. In practice, some
/// devices will not respond to requests with an MX value above some threshold
/// (but lower than the maximum threshold) because of resources it may not want
/// to tie up.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct MX(pub u8);

impl MX {
    pub fn new(wait_bound: u8) -> SSDPResult<MX> {
        if !(MX_HEADER_MIN..=MX_HEADER_MAX).contains(&wait_bound) {
            Err(SSDPErrorKind::InvalidHeader(MX_HEADER_NAME, "Supplied Wait Bound Is Out Of Bounds").into())
        } else {
            Ok(MX(wait_bound))
        }
    }
}

impl Header for MX {
    fn header_name() -> &'static str {
        MX_HEADER_NAME
    }

    fn parse_header(raw: &[Vec<u8>]) -> error::Result<Self> {
        if raw.len() != 1 {
            return Err(Error::Header);
        }

        let cow_string = String::from_utf8_lossy(&raw[0][..]);

        match cow_string.parse::<u8>() {
            Ok(n) if (MX_HEADER_MIN..=MX_HEADER_MAX).contains(&n) => Ok(MX(n)),
            _ => Err(Error::Header),
        }
    }
}

impl HeaderFormat for MX {
    fn fmt_header(&self, fmt: &mut Formatter) -> Result {
        fmt.write_fmt(format_args!("{}", self.0))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hyper::header::Header;

    use super::MX;

    #[test]
    fn positive_lower_bound() {
        let mx_lower_header = &[b"1"[..].to_vec()];

        match MX::parse_header(mx_lower_header) {
            Ok(n) if n == MX(1) => (),
            _ => panic!("Failed To Accept 1 As MX Value"),
        };
    }

    #[test]
    fn positive_inner_bound() {
        let mx_inner_header = &[b"5"[..].to_vec()];

        match MX::parse_header(mx_inner_header) {
            Ok(n) if n == MX(5) => (),
            _ => panic!("Failed To Accept 5 As MX Value"),
        };
    }

    #[test]
    fn positive_upper_bound() {
        let mx_upper_header = &[b"120"[..].to_vec()];

        match MX::parse_header(mx_upper_header) {
            Ok(n) if n == MX(120) => (),
            _ => panic!("Failed To Accept 120 As MX Value"),
        };
    }

    #[test]
    #[should_panic]
    fn negative_decimal_bound() {
        let mx_decimal_header = &[b"0.5"[..].to_vec()];

        MX::parse_header(mx_decimal_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_negative_bound() {
        let mx_negative_header = &[b"-5"[..].to_vec()];

        MX::parse_header(mx_negative_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_too_high_bound() {
        let mx_too_high_header = &[b"121"[..].to_vec()];

        MX::parse_header(mx_too_high_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_zero_bound() {
        let mx_zero_header = &[b"0"[..].to_vec()];

        MX::parse_header(mx_zero_header).unwrap();
    }
}
