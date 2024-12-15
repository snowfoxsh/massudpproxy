mod router;

use tokio::io;
use crate::router::Router;

fn bind_ports() {
    
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // create our router manager
    let router = Router::new().route(
        "127.0.0.1:5000".parse().unwrap(),
        "127.0.0.1:6000".parse().unwrap(),
    );
    
    println!("{:?}", router.routes());
    
    Ok(())
}

// map
// inputs -> outputs
// inputs:x-y --1:1-> outputs:x-y
// ^^^ i need to handle multiple rules sets for these

// create a table of where things need to go

