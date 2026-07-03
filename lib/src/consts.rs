pub const SOCKS_VERSION: u8 = 0x05;
pub const RSV: u8 = 0x00; // reserved (always 0)

pub mod auth {
    pub const NO_AUTH: u8 = 0x00;
    pub const AUTH: u8 = 0x02;
    pub const VERSION: u8 = 0x01;
}

pub mod connect {
    pub const CMD: u8 = 0x01;
    pub const ATYP_IPV4: u8 = 0x01;
    pub const ATYP_DOMAINNAME: u8 = 0x03;
    pub const ATYP_IPV6: u8 = 0x04;
}

pub mod reply {
    pub const SUCCESS: u8 = 0x00;
    pub const FAILURE: u8 = 0x01;
    pub const NO_ACCEPTABLE_METHOD: u8 = 0xFF;
    pub const BND_ADDR: &[u8] = &[0x00, 0x00, 0x00, 0x00];
    pub const BND_PORT: &[u8] = &[0x00, 0x00];
}