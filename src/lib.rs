use std::{net::{SocketAddr, UdpSocket}, option};
use tokio::{net, sync::broadcast};
use tokio::spawn;
use tokio::time::Duration;
use std::net::ToSocketAddrs;



struct Client {
    //some field of websocket connection client.
    //authentication methods blah blah blah
    // tokens
    port: i8,
}
pub struct Dominator {
    clients: Vec<Client>,
    server_address: String,
    client_host_port: i32,
    broadcast_address: String,
}   

impl Dominator {
    pub async fn new() -> Self //no checking for shitty address rn.
    {
        println!("starting server up!");
        //initialize dominator fields,
        let clients: Vec<Client> = Vec::new();

        // find current local IP address
        let server_address = String::from("192.168.0.1");
        let client_host_port = 1;
        let broadcast_address = "239.255.255.250:1901".to_string();
        let argument2 = broadcast_address.clone();

        tokio::spawn(async move {
            Dominator::start_broadcast("192.168.0.1".to_string(), argument2).await;
        });

       Dominator {clients, server_address, client_host_port, broadcast_address}
    }

    //function that setups up a handler loop that accepts incoming connections then diverts them to
    //another assigned port by sending them the new IP address?.
    async fn connectionHandler() -> String{


        "a".to_string()
    }

    async fn start_broadcast(message: String, broadcast_address: String){
        println!("starting broadcasting!");

        let socket = UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind to address");
        socket.set_multicast_loop_v4(true).expect("Couldn't set multicast loop");

        let message = format!("Trade Server at {}",message);
        println!("started broadcoasting ");

        loop {
            socket.send_to(message.as_bytes(), &broadcast_address).expect("Couldn't send data");
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }


}
