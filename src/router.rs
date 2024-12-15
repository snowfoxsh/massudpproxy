use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct Router {
    routes: HashMap<SocketAddr, SocketAddr>,
}

#[allow(dead_code)]
impl Router {
    pub fn new() -> Router {
        Router {
            routes: HashMap::new(),
        }
    }

    pub fn add_route(&mut self, input: SocketAddr, output: SocketAddr) {
        self.routes.insert(input, output);
    }

    pub fn route(mut self, input: SocketAddr, output: SocketAddr) -> Self {
        self.add_route(input, output);
        self
    }

    pub fn add_direct_routes<PortRange: IntoIterator<Item = u16>>(
        &mut self,
        input_ip: IpAddr,
        output_ip: IpAddr,
        port_range: PortRange,
    ) {
        port_range.into_iter().for_each(|port: u16| {
            self.add_route(
                SocketAddr::new(input_ip, port),
                SocketAddr::new(output_ip, port),
            );
        });
    }

    pub fn direct_routes<PortRange: IntoIterator<Item = u16>>(
        mut self,
        input_ip: IpAddr,
        output_ip: IpAddr,
        port_range: PortRange,
    ) -> Self {
        self.add_direct_routes(input_ip, output_ip, port_range);
        self
    }

    // returns None if it is impossible
    pub fn add_offset_routes<PortRange: IntoIterator<Item = u16>>(
        &mut self,
        input_ip: IpAddr,
        input_port_range: PortRange,
        output_ip: IpAddr,
        output_port_range: PortRange,
    ) -> Option<()> {
        // Use iterators to simultaneously iterate over both port ranges
        let mut input_ports = input_port_range.into_iter();
        let mut output_ports = output_port_range.into_iter();

        loop {
            match (input_ports.next(), output_ports.next()) {
                (Some(input_port), Some(output_port)) => {
                    self.add_route(
                        SocketAddr::new(input_ip, input_port),
                        SocketAddr::new(output_ip, output_port),
                    );
                }
                (None, None) => break, // both ranges are fully iterated
                _ => return None,      // mismatched range lengths
            }
        }

        Some(())
    }

    pub fn offset_routes<PortRange: IntoIterator<Item = u16>>(
        mut self,
        input_ip: IpAddr,
        input_port_range: PortRange,
        output_ip: IpAddr,
        output_port_range: PortRange,
    ) -> Option<Self> {
        self.add_offset_routes(input_ip, input_port_range, output_ip, output_port_range);
        Some(self)
    }

    /// method that calculates the ports that need to be bound based on routes
    /// todo: rename this
    pub fn required_ports(&self) -> HashSet<u16> {
        // self.routes.iter().map(|route| route.key().port()).collect()
        self.routes.keys().map(|key| key.port()).collect()
    }

    pub fn required_sockets(routes: &HashMap<SocketAddr, SocketAddr>) -> HashSet<SocketAddr> {
    //     self.routes.iter().map(|route| *route.key()).collect()
        routes.keys().map(|key| *key).collect()
    }
    
    pub fn get_routes(&self) -> &HashMap<SocketAddr, SocketAddr> {
        &self.routes
    }

    pub fn to_routes(self) -> HashMap<SocketAddr, SocketAddr> {
        self.routes
    }
}
