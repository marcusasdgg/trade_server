use std::{net::{SocketAddr, UdpSocket}, option};
use tokio::net;
use tokio::spawn;
use tokio::time::Duration;

struct Client {
    //some field of websocket connection client.
    //authentication methods blah blah blah
    // tokens
}
pub struct Dominator {
    //clients: Vec<Client>,
}   

impl Dominator {
    pub async fn new(optional_address: &str) //no checking for shitty address rn.
    {
        println!("starting server up!");
        let add = optional_address.to_string();
        tokio::spawn(async {
            Dominator::start_broadcast(add).await;
        });
       
    }

    async fn start_broadcast(address: String){
        println!("starting broadcasting!");
        let multiaddress = "239.255.255.250:1900".to_string();

        let socket = UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind to address");
        socket.set_multicast_loop_v4(true).expect("Couldn't set multicast loop");

        let message = format!("Trade Server at {}",address);
        println!("started broadcoasting ");

        loop {
            socket.send_to(message.as_bytes(), multiaddress.clone()).expect("Couldn't send data");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }


}
