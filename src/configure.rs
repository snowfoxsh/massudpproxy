use std::net::{IpAddr, SocketAddr};
use std::ops::Range;
use tokio::io;
use serde_derive::{Deserialize};
use crate::port_range::PortRange;
use crate::router::Router;

#[derive(Deserialize, Debug)]
pub struct Config {
    routes: Vec<RouteConfig>
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum RouteConfig {
    SinglePort {
        local: SocketAddr,
        remote: SocketAddr,
    },
    SimpleRange {
        local_addr: IpAddr,
        remote_addr: IpAddr,
        port_range: PortRange,
    },
    ComplexPortRange {
        local_addr: IpAddr,
        remote_addr: IpAddr,
        local_port_range: PortRange,
        remote_port_range: PortRange,
    }
}

// #[derive(Deserialize, Debug)]
// struct RouteConfig {
//     local: SocketAddr,
//     remote: SocketAddr,
// }

impl Config {
    pub async fn load_file(path: &str) -> io::Result<Config> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(config)
    }

    pub fn router(&self) -> Router {
        let router = Router::new();
        
        for route in &self.routes {
            match route {
                RouteConfig::SinglePort { local, remote } => {
                    router.add_route(*local, *remote);
                }
                RouteConfig::SimpleRange { local_addr: local, remote_addr: remote, port_range } => {
                    router.add_direct_routes( *local, *remote, port_range.clone())
                }
                RouteConfig::ComplexPortRange { local_addr: local, remote_addr: remote, local_port_range, remote_port_range } => {
                    router.add_offset_routes(*local, local_port_range.clone(), *remote, remote_port_range.clone())
                        .expect(format!("local port range {local_port_range:?} must be the same length as remote port range {remote_port_range:?}").as_str());
                }
            }

            // router.add_route(route.local, route.remote);
        }
        
        router
    }
}