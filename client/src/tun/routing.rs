use netlink_packet_core::{NLM_F_ACK, NLM_F_CREATE, NLM_F_EXCL, NLM_F_REQUEST, NetlinkHeader, NetlinkMessage, NetlinkPayload};
use netlink_packet_route::route::{RouteAttribute, RouteHeader, RouteMessage, RouteProtocol, RouteAddress, RouteScope, RouteType};
use netlink_packet_route::rule::{RuleAction, RuleAttribute, RuleHeader, RuleMessage};
use netlink_packet_route::{AddressFamily, RouteNetlinkMessage};
use netlink_sys::{Socket, SocketAddr, protocols::NETLINK_ROUTE};

use std::net::Ipv4Addr;

use crate::prelude::*;

const RT_TABLE_LOCAL: u8 = 255; //* /etc/iproute2/rt_tables
const _RT_TABLE_MAIN: u32 = 254;
const _RT_TABLE_DEFAULT: u32 = 253;

const TABLE_ID: u32 = 12345;

pub const FAKE_IP_POOL: Ipv4Addr = Ipv4Addr::new(100, 64, 0, 0);
const FAKE_IP_PREFIX: u8 = 10;

enum Action {
    Add,
    Delete
}

pub struct Routing {
    tun_index: u32,
    socket: Socket
}

impl Routing {
    pub fn new(tun_index: u32) -> Result<Self, AppError> {
        let mut socket = Socket::new(NETLINK_ROUTE)?;
        socket.bind_auto()?;
        socket.connect(&SocketAddr::new(0, 0))?;

        Ok(Self { tun_index, socket })
    }

    pub fn setup(&self) -> Result<(), AppError> {
        self.add_default_route()?;
        self.add_fake_ip_route()?;

        self.add_default_rule()?;
        self.remove_local_rule()
    }

    pub fn cleanup(&self) -> Result<(), AppError> {
        self.remove_default_route()?;
        self.remove_fake_ip_route()?;

        self.remove_default_rule()?;
        self.add_local_rule()
    }

    fn add_default_route(&self) -> Result<(), AppError> {
        let route = self.default_route();
        let msg = Self::wrap_route_to_msg(route, &Action::Add);
        self.send(msg).map_err(|e| AppError::ModeTun(format!("failed to add default route | error: {e}")))
    }

    fn add_fake_ip_route(&self) -> Result<(), AppError> {
        let route = self.fake_ip_route();
        let msg = Self::wrap_route_to_msg(route, &Action::Add);
        self.send(msg).map_err(|e| AppError::ModeTun(format!("failed to add fake ip route | error: {e}")))
    }

    fn add_default_rule(&self) -> Result<(), AppError> {
        let rule = Self::default_rule();
        let msg = Self::wrap_rule_to_msg(rule, &Action::Add);
        self.send(msg).map_err(|e| AppError::ModeTun(format!("failed to add default rule | error: {e}")))
    }

    fn add_local_rule(&self) -> Result<(), AppError> {
        let rule = Self::local_rule();
        let msg = Self::wrap_rule_to_msg(rule, &Action::Add);
        self.send(msg).map_err(|e| AppError::ModeTun(format!("failed to add local rule | error: {e}")))
    }

    fn remove_default_route(&self) -> Result<(), AppError> {
        let route = self.default_route();
        let msg = Self::wrap_route_to_msg(route, &Action::Delete);
        self.send(msg).map_err(|e| AppError::ModeTun(format!("failed to remove default route | error: {e}")))
    }

    fn remove_fake_ip_route(&self) -> Result<(), AppError> {
        let route = self.fake_ip_route();
        let msg = Self::wrap_route_to_msg(route, &Action::Delete);
        self.send(msg).map_err(|e| AppError::ModeTun(format!("failed to remove fake ip route | error: {e}")))
    }

    fn remove_default_rule(&self) -> Result<(), AppError> {
        let rule = Self::default_rule();
        let msg = Self::wrap_rule_to_msg(rule, &Action::Delete);
        self.send(msg).map_err(|e| AppError::ModeTun(format!("failed to remove default rule | error: {e}")))
    }

    fn remove_local_rule(&self) -> Result<(), AppError> {
        let rule = Self::local_rule();
        let msg = Self::wrap_rule_to_msg(rule, &Action::Delete);
        self.send(msg).map_err(|e| AppError::ModeTun(format!("failed to remove local rule | error: {e}")))
    }

    fn send(&self, mut msg: NetlinkMessage<RouteNetlinkMessage>) -> Result<(), AppError> {
        msg.finalize();
        let mut buf = vec![0; msg.buffer_len()];
        msg.serialize(&mut buf);
        
        debug!(msg=?msg.payload);

        self.socket.send(&buf, 0).map_err(|e| AppError::ModeTun(format!("failed to send netlink message | error: {e}")))?;

        let mut response = vec![0u8; 4096];
        let len = self.socket.recv(&mut response, 0)?;
        if len >= 20 { //16 bytes header + 4 bytes - error_code
            let error_code = i32::from_ne_bytes(response[16..20].try_into()?);
            if error_code < 0 { return Err(AppError::ModeTun(format!("Routing error | error code: {error_code}"))); }
        }

        Ok(())
    }

    //* default dev tun0 table 12345 proto static
    fn default_route(&self) -> RouteMessage {
        let mut route = RouteMessage::default();
        route.header = RouteHeader {
            address_family: AddressFamily::Inet,
            protocol: RouteProtocol::Static,
            scope: RouteScope::Universe,
            kind: RouteType::Unicast,
            ..Default::default()
        };
        route.attributes = vec![
            RouteAttribute::Table(TABLE_ID),
            RouteAttribute::Oif(self.tun_index),
            RouteAttribute::Destination(RouteAddress::Inet(Ipv4Addr::UNSPECIFIED))
        ];
        route
    }

    //* 100.64.0.0/10 dev tun0 proto static
    fn fake_ip_route(&self) -> RouteMessage {
        let mut route = RouteMessage::default();
        route.header = RouteHeader {
            address_family: AddressFamily::Inet,
            protocol: RouteProtocol::Static,
            scope: RouteScope::Universe,
            kind: RouteType::Unicast,
            destination_prefix_length: FAKE_IP_PREFIX,
            ..Default::default()
        };
        route.attributes = vec![
            RouteAttribute::Table(TABLE_ID),
            RouteAttribute::Oif(self.tun_index),
            RouteAttribute::Destination(RouteAddress::Inet(FAKE_IP_POOL))
        ];
        route
    }

    //* from all lookup 12345
    fn default_rule() -> RuleMessage {
        let mut rule = RuleMessage::default();
        rule.header = RuleHeader {
            family: AddressFamily::Inet,
            action: RuleAction::ToTable,
            ..Default::default()
        };
        rule.attributes = vec![
            RuleAttribute::Table(TABLE_ID),
            RuleAttribute::Protocol(RouteProtocol::Kernel),
            RuleAttribute::Priority(1000),
        ];
        rule
    }

    //* 0:	from all lookup local
    fn local_rule() -> RuleMessage {
        let mut rule = RuleMessage::default();
        rule.header = RuleHeader {
            family: AddressFamily::Inet,
            action: RuleAction::ToTable,
            table: RT_TABLE_LOCAL,
            ..Default::default()
        };
        rule.attributes = vec![
            RuleAttribute::Table(u32::from(RT_TABLE_LOCAL)),
            RuleAttribute::Priority(0),
            RuleAttribute::Protocol(RouteProtocol::Kernel),
        ];
        rule
    }

    fn wrap_route_to_msg(rule: RouteMessage, action: &Action) -> NetlinkMessage<RouteNetlinkMessage> {
        match action {
            Action::Add => NetlinkMessage::new(Self::msg_header(action), NetlinkPayload::from(RouteNetlinkMessage::NewRoute(rule))),
            Action::Delete => NetlinkMessage::new(Self::msg_header(action), NetlinkPayload::from(RouteNetlinkMessage::DelRoute(rule))),
        }
    }

    fn wrap_rule_to_msg(rule: RuleMessage, action: &Action) -> NetlinkMessage<RouteNetlinkMessage> {
        match action {
            Action::Add => NetlinkMessage::new(Self::msg_header(action), NetlinkPayload::from(RouteNetlinkMessage::NewRule(rule))),
            Action::Delete => NetlinkMessage::new(Self::msg_header(action), NetlinkPayload::from(RouteNetlinkMessage::DelRule(rule))),
        }
    }

    fn msg_header(action: &Action) -> NetlinkHeader {
        let mut msg_header = NetlinkHeader::default();
        msg_header.flags = match action {
            Action::Add => NLM_F_REQUEST | NLM_F_CREATE | NLM_F_EXCL | NLM_F_ACK,
            Action::Delete => NLM_F_REQUEST | NLM_F_ACK,
        };
        msg_header
    }
}