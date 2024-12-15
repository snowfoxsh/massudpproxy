use std::collections::HashSet;
use std::sync::Arc;
use dashmap::DashMap;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Default)]
pub struct Router {
    routes: Arc<DashMap<SocketAddr, SocketAddr>>
}


impl Router {
    pub fn new() -> Router {
        Router {
            routes: Arc::new(DashMap::new())
        }
    }
    
    pub fn add_route(&self, input: SocketAddr, output:SocketAddr) {
        self.routes.insert(input, output);
    }

    pub fn route(self, input: SocketAddr, output:SocketAddr) -> Self {
        self.add_route(input, output);
        self
    }
    
    
    pub fn add_direct_routes<PortRange: Iterator<Item = u16>>(&self, input_ip: IpAddr, output_ip: IpAddr, port_range: PortRange) {
        port_range.for_each(|port: u16| {
            self.add_route(
                SocketAddr::new(input_ip, port),
                SocketAddr::new(output_ip, port)
            );
        });
    }
    
    pub fn direct_routes<PortRange: Iterator<Item = u16>>(self, input_ip: IpAddr, output_ip: IpAddr, port_range: PortRange) -> Self {
        self.add_direct_routes(input_ip, output_ip, port_range);
        self
    }
    
    /// method that calculates the ports that need to be bound based on routes
    /// todo: rename this
    pub fn required_ports(&self) -> HashSet<u16> {
        self.routes.iter()
            .map(|route| route.key().port())
            .collect()
    }
    
    pub fn required_sockets(&self) -> HashSet<SocketAddr> {
        self.routes.iter()
            .map(|route| *route.key())
            .collect()
    }
    
    pub fn routes(&self) -> Arc<DashMap<SocketAddr, SocketAddr>> {
        Arc::clone(&self.routes)
    }
}
