use std::time::Duration;

use tokio::main;
use trade_server::Dominator;

#[tokio::main]
async fn main() -> Result<(),()> {
    println!("Hello, world!");
    let s = Dominator::new().await;
    tokio::time::sleep(Duration::from_secs(2000)).await;
    Ok(())
}
