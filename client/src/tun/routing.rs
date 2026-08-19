use netlink_packet_core::{NLM_F_ACK, NLM_F_CREATE, NLM_F_EXCL, NLM_F_REQUEST, NetlinkHeader, NetlinkMessage, NetlinkPayload};
use netlink_packet_route::route::{RouteAttribute, RouteHeader, RouteMessage, RouteProtocol, RouteAddress, RouteScope, RouteType};
use netlink_packet_route::rule::{RuleAction, RuleAttribute, RuleHeader, RuleMessage};
use netlink_packet_route::{AddressFamily, RouteNetlinkMessage};
use netlink_sys::{Socket, SocketAddr, protocols::NETLINK_ROUTE};
use std::net::Ipv4Addr;

use crate::prelude::*;

const RT_TABLE_MAIN: u32 = 254; //* /etc/iproute2/rt_tables
const _RT_TABLE_LOCAL: u8 = 255;
const _RT_TABLE_DEFAULT: u32 = 253;

const TABLE_ID: u32 = 12345;
const PRIORITY: u32 = 1000;

enum Action {
    Add,
    Delete
}

pub struct Routing {
    tun_address: Ipv4Addr,
    server_address: Ipv4Addr,
    tun_index: u32,
    socket: Socket,
}

impl Routing {
    pub fn new(config: &Config, tun_index: u32) -> Result<Self, AppError> {
        let mut socket = Socket::new(NETLINK_ROUTE)?;
        socket.bind_auto()?;
        socket.connect(&SocketAddr::new(0, 0))?;

        Ok(Self { 
            tun_address: config.address, 
            server_address: *config.server.ip(), 
            tun_index, 
            socket
        })
    }

    pub fn setup(&self) -> Result<(), AppError> {
        self.add_default_route()?;

        self.add_default_rule()?;
        self.add_socks5_rule()?;
        self.add_dns_forwarding_rule()
    }

    pub fn cleanup(&self) -> Result<(), AppError> {
        self.remove_default_route()?;

        self.remove_default_rule()?;
        self.remove_socks5_rule()?;
        self.remove_dns_forwarding_rule()
    }

    fn add_default_route(&self) -> Result<(), AppError> {
        let route = self.default_route();
        let msg = Self::wrap_route_to_msg(route, &Action::Add);
        self.send(msg).map_err(|e| AppError::Routing(format!("failed to add default route | error: {e}")))
    }

    fn add_default_rule(&self) -> Result<(), AppError> {
        let rule = Self::default_rule();
        let msg = Self::wrap_rule_to_msg(rule, &Action::Add);
        self.send(msg).map_err(|e| AppError::Routing(format!("failed to add default rule | error: {e}")))
    }

    fn add_socks5_rule(&self) -> Result<(), AppError> {
        let rule = self.socks5_rule();
        let msg = Self::wrap_rule_to_msg(rule, &Action::Add);
        self.send(msg).map_err(|e| AppError::Routing(format!("failed to add socks5 rule | error: {e}")))
    }

    fn add_dns_forwarding_rule(&self) -> Result<(), AppError> {
        let ipt = iptables::new(false).map_err(|e| AppError::Routing(e.to_string()))?;
        if ipt.exists("nat", "OUTPUT", &self.dns_forwarding_rule()).unwrap_or(false) {
            return Ok(());
        }
        ipt.append("nat", "OUTPUT", &self.dns_forwarding_rule())
            .map_err(|e| AppError::Routing(format!("failed to add dns forwarding rule | error: {e}")))
    }

    fn remove_default_route(&self) -> Result<(), AppError> {
        let route = self.default_route();
        let msg = Self::wrap_route_to_msg(route, &Action::Delete);
        self.send(msg).map_err(|e| AppError::Routing(format!("failed to remove default route | error: {e}")))
    }

    fn remove_default_rule(&self) -> Result<(), AppError> {
        let rule = Self::default_rule();
        let msg = Self::wrap_rule_to_msg(rule, &Action::Delete);
        self.send(msg).map_err(|e| AppError::Routing(format!("failed to remove default rule | error: {e}")))
    }

    fn remove_socks5_rule(&self) -> Result<(), AppError> {
        let rule = self.socks5_rule();
        let msg = Self::wrap_rule_to_msg(rule, &Action::Delete);
        self.send(msg).map_err(|e| AppError::Routing(format!("failed to remove socks5 rule | error: {e}")))
    }

    fn remove_dns_forwarding_rule(&self) -> Result<(), AppError> {
        let ipt = iptables::new(false).map_err(|e| AppError::Routing(e.to_string()))?;
        if !ipt.exists("nat", "OUTPUT", &self.dns_forwarding_rule()).unwrap_or(false) {
            return Ok(());
        }
        ipt.delete("nat", "OUTPUT", &self.dns_forwarding_rule())
            .map_err(|e| AppError::Routing(format!("failed to remove dns forwarding rule | error: {e}")))
    }

    fn send(&self, mut msg: NetlinkMessage<RouteNetlinkMessage>) -> Result<(), AppError> {
        msg.finalize();
        let mut buf = vec![0; msg.buffer_len()];
        msg.serialize(&mut buf);
        
        debug!(msg=?msg.payload);

        self.socket.send(&buf, 0).map_err(|e| AppError::Routing(format!("failed to send netlink message | error: {e}")))?;

        let mut response = vec![0u8; 4096];
        let len = self.socket.recv(&mut response, 0)?;
        if len >= 20 { //16 bytes header + 4 bytes - error_code
            let error_code = i32::from_ne_bytes(response[16..20].try_into()?);
            if error_code < 0 { return Err(AppError::Routing(format!("netlink error code: {error_code}"))); }
        }

        Ok(())
    }

    fn dns_forwarding_rule(&self) -> String {
        format!("-p udp --dport 53 -j DNAT --to-destination {}:53", self.tun_address)
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

    //* from all lookup 12345 priority 1000
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
            RuleAttribute::Priority(PRIORITY),
        ];
        rule
    }

    //* from all to {server_ip} lookup main priority 999
    fn socks5_rule(&self) -> RuleMessage {
        let mut rule = RuleMessage::default();
        rule.header = RuleHeader {
            family: AddressFamily::Inet,
            dst_len: 32,
            action: RuleAction::ToTable,
            ..Default::default()
        };
        rule.attributes = vec![
            RuleAttribute::Table(RT_TABLE_MAIN),
            RuleAttribute::Destination(self.server_address.into()),
            RuleAttribute::Protocol(RouteProtocol::Static),
            RuleAttribute::Priority(PRIORITY-1),
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