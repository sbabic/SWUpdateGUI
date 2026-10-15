// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains utility functions for network settings

use crate::system::run_script;
use std::ffi::CStr;
use std::net::Ipv4Addr;

const SCRIPT_SET_GATEWAY: &str = "/usr/bin/recovery-edit-default-gateway.sh";
const SCRIPT_SET_INTERFACE_DHCP: &str = "/usr/bin/recovery-set-dhcp.sh";
const SCRIPT_SET_INTERFACE_STATIC: &str = "/usr/bin/recovery-set-static.sh";
const SCRIPT_GET_DHCP_STATUS: &str = "/usr/bin/recovery-get-dhcp-status.sh";
const SCRIPT_CHECK_BRIDGE_INTERFACE: &str = "/usr/bin/recovery-check-bridge-interface.sh";

const ROUTES_FILE: &str = "/proc/net/route";

pub const DEFAULT_GATEWAY_STR: &str = "default-gateway";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ipv4Info {
    pub name: String,
    pub address: String,
    pub netmask: String,
    pub is_dhcp: bool,
}

impl Ipv4Info {
    pub fn is_default_gateway(&self) -> bool {
        self.name == DEFAULT_GATEWAY_STR
    }

    /// Text for the dhcp field: empty for the default gateway, otherwise "yes" or "no".
    pub fn dhcp_text(&self) -> &'static str {
        match (self.is_default_gateway(), self.is_dhcp) {
            (true, _) => "",
            (false, true) => "yes",
            (false, false) => "no",
        }
    }
}

fn is_dhcp(address: &str) -> bool {
    if address.is_empty() {
        return false;
    }
    run_script(SCRIPT_GET_DHCP_STATUS, &[format!("--address={address}")], &[0, 1]) == 0
}

fn is_bridge_port(interface: &str) -> bool {
    run_script(SCRIPT_CHECK_BRIDGE_INTERFACE, &[format!("--interface={interface}")], &[0, 1]) == 1
}

/// Convert a hex value from /proc/net/route to a dotted IP address,
/// e.g. "FE01A8C0" > "192.168.1.254"
pub fn hex_to_dotted_address(hex: &str) -> Option<String> {
    let mut bytes = [0u8; 4];
    let mut rest = hex;

    for byte in bytes.iter_mut() {
        let digits = rest
            .char_indices()
            .take(2)
            .take_while(|(_, c)| c.is_ascii_hexdigit())
            .count();
        if digits == 0 {
            return None;
        }
        *byte = u8::from_str_radix(&rest[..digits], 16).ok()?;
        rest = &rest[digits..];
    }

    Some(Ipv4Addr::new(bytes[3], bytes[2], bytes[1], bytes[0]).to_string())
}

/// Find the default gateway in the contents of /proc/net/route
pub fn parse_default_gateway(route_table: &str) -> Option<String> {
    route_table
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace().skip(1);
            match (fields.next(), fields.next()) {
                (Some("00000000"), Some(gateway)) => Some(gateway),
                _ => None,
            }
        })
        .next()
        .and_then(hex_to_dotted_address)
        .filter(|address| address != "0.0.0.0")
}

/// Get default gateway information
pub fn default_gateway_info() -> Ipv4Info {
    let address = match std::fs::read_to_string(ROUTES_FILE) {
        Ok(table) => parse_default_gateway(&table).unwrap_or_default(),
        Err(err) => {
            log::warn!("Unable to open {ROUTES_FILE}: {err}");
            String::new()
        }
    };

    Ipv4Info {
        name: DEFAULT_GATEWAY_STR.into(),
        address,
        ..Default::default()
    }
}

/// Number of leading 1-bits of a netmask, or None if the 1-bits are not continuous.
pub fn netmask_prefix_length(netmask: Ipv4Addr) -> Option<u32> {
    let value = u32::from(netmask);
    let ones = value.leading_ones();
    (ones + value.trailing_zeros() >= 32).then_some(ones)
}

/// Convert dotted IP-address & netmask to cidr format, e.g. "192.168.1.14/24"
pub fn to_cidr(address: &str, netmask: &str) -> Option<String> {
    let prefix = netmask
        .trim()
        .parse()
        .ok()
        .and_then(netmask_prefix_length);

    match prefix {
        Some(prefix) => Some(format!("{address}/{prefix}")),
        None => {
            log::error!("Invalid netmask detected: {netmask}");
            None
        }
    }
}

/// Reconfigure a network interface with static configuration.
pub fn set_static_configuration(interface: &str, address: &str, netmask: &str) -> bool {
    let Some(cidr) = to_cidr(address, netmask) else {
        return false;
    };

    let args = [format!("--interface={interface}"), format!("--ip={cidr}")];
    run_script(SCRIPT_SET_INTERFACE_STATIC, &args, &[0, 1]) == 0
}

/// Reconfigure a network interface as a dhcp client.
pub fn reconfigure_as_dhcp_client(interface: &str) -> bool {
    run_script(SCRIPT_SET_INTERFACE_DHCP, &[format!("--interface={interface}")], &[0, 1]) == 0
}

pub fn delete_gateway() -> bool {
    run_script(SCRIPT_SET_GATEWAY, &["--action=delete".into()], &[0, 1]) == 0
}

pub fn set_gateway_address(address: &str) -> bool {
    let args = ["--action=set".into(), format!("--ip={address}")];
    run_script(SCRIPT_SET_GATEWAY, &args, &[0, 1]) == 0
}

/// Read an IPv4 address from a sockaddr, ignoring other families and 0.0.0.0
///
/// # Safety
/// `addr` must be null or point to a valid sockaddr.
unsafe fn ipv4_from_sockaddr(addr: *const libc::sockaddr) -> Option<String> {
    if addr.is_null() || i32::from((*addr).sa_family) != libc::AF_INET {
        return None;
    }
    let addr = &*(addr as *const libc::sockaddr_in);
    let ip = Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr));
    (!ip.is_unspecified()).then(|| ip.to_string())
}

/// Create the list of configured interfaces that exist and are not bridge ports,
/// in the order reported by the system.
pub fn interface_list(configured: &[String]) -> Vec<Ipv4Info> {
    let mut ifaddrs: *mut libc::ifaddrs = std::ptr::null_mut();

    if unsafe { libc::getifaddrs(&mut ifaddrs) } == -1 {
        log::warn!(
            "Failed to get network information: {}",
            std::io::Error::last_os_error()
        );
        return Vec::new();
    }

    let mut interfaces: Vec<Ipv4Info> = Vec::new();
    let mut bridge_ports: Vec<String> = Vec::new();

    let mut current = ifaddrs;
    while !current.is_null() {
        // SAFETY: getifaddrs() returned a valid linked list, freed below.
        let iface = unsafe { &*current };
        current = iface.ifa_next;

        let name = unsafe { CStr::from_ptr(iface.ifa_name) }.to_string_lossy();
        if !configured.iter().any(|c| *c == name) || bridge_ports.iter().any(|b| *b == name) {
            continue;
        }

        let index = match interfaces.iter().position(|i| i.name == name) {
            Some(index) => index,
            None if is_bridge_port(&name) => {
                bridge_ports.push(name.into_owned());
                continue;
            }
            None => {
                interfaces.push(Ipv4Info {
                    name: name.into_owned(),
                    ..Default::default()
                });
                interfaces.len() - 1
            }
        };

        let info = &mut interfaces[index];
        if let Some(address) = unsafe { ipv4_from_sockaddr(iface.ifa_addr) } {
            info.address = address;
            if let Some(netmask) = unsafe { ipv4_from_sockaddr(iface.ifa_netmask) } {
                info.netmask = netmask;
            }
        }
    }

    unsafe { libc::freeifaddrs(ifaddrs) };

    for info in &mut interfaces {
        info.is_dhcp = is_dhcp(&info.address);
    }

    interfaces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_to_ipv4_conversion() {
        assert_eq!(hex_to_dotted_address("FE01A8C0").as_deref(), Some("192.168.1.254"));
        assert_eq!(hex_to_dotted_address("AXELOT"), None);
        assert_eq!(hex_to_dotted_address("0000001").as_deref(), Some("1.0.0.0"));
        assert_eq!(hex_to_dotted_address("+-0039403"), None);
    }

    #[test]
    fn convert_to_cidr() {
        assert_eq!(to_cidr("192.168.1.14", "255.255.255.0").as_deref(), Some("192.168.1.14/24"));
        assert_eq!(to_cidr("ABCD", "255.0.0.0").as_deref(), Some("ABCD/8"));
        assert_eq!(to_cidr("1.1.1.1", "220.128.0.0"), None);
        assert_eq!(to_cidr("192.168.1.14", "255.254.0.0").as_deref(), Some("192.168.1.14/15"));
        assert_eq!(to_cidr("192.168.1.14", "128.0.0.0").as_deref(), Some("192.168.1.14/1"));
        assert_eq!(to_cidr("192.168.1.14", "255.255.224.0").as_deref(), Some("192.168.1.14/19"));
        assert_eq!(to_cidr("192.168.1.14", "255.255.255.252").as_deref(), Some("192.168.1.14/30"));
        assert_eq!(to_cidr("192.168.1.14", "255.255.255.255").as_deref(), Some("192.168.1.14/32"));
        assert_eq!(to_cidr("192.168.1.14", "255.255.248.0").as_deref(), Some("192.168.1.14/21"));
        assert_eq!(to_cidr("192.168.1.14", "0.0.0.0").as_deref(), Some("192.168.1.14/0"));
        assert_eq!(to_cidr("192.168.1.14", "not a netmask"), None);
    }

    #[test]
    fn default_gateway_from_route_table() {
        let table = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\n\
                     eth0\t0001A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\n\
                     eth0\t00000000\tFE01A8C0\t0003\t0\t0\t0\t00000000\n";
        assert_eq!(parse_default_gateway(table).as_deref(), Some("192.168.1.254"));

        let no_default = "Iface\tDestination\tGateway\n\
                          eth0\t0001A8C0\t00000000\n";
        assert_eq!(parse_default_gateway(no_default), None);

        let zero = "eth0\t00000000\t00000000\n";
        assert_eq!(parse_default_gateway(zero), None);
    }

    #[test]
    fn dhcp_text() {
        let mut info = Ipv4Info { name: "eth0".into(), ..Default::default() };
        assert_eq!(info.dhcp_text(), "no");
        info.is_dhcp = true;
        assert_eq!(info.dhcp_text(), "yes");
        info.name = DEFAULT_GATEWAY_STR.into();
        assert_eq!(info.dhcp_text(), "");
        assert!(info.is_default_gateway());
    }
}
