use std::{net::{SocketAddr, UdpSocket}, option};
use tokio::{net, sync::broadcast};
use tokio::spawn;
use tokio::time::Duration;
use std::net::ToSocketAddrs;
use if_addrs::get_if_addrs;
use std::io::{self, Write};
use std::net::{TcpListener, TcpStream};


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
        let clients = Vec::new();

        // find current local IP address
        let server_address = String::from("192.168.0.1");
        let broadcast_address = "239.255.255.250:1901".to_string();
        let argument2 = broadcast_address.clone();

        let mut broadcast_address : Option<String> = None;
        for iface in if_addrs::get_if_addrs().unwrap() {
           if iface.addr.ip().to_string().contains("192.168.0."){
            broadcast_address = Some(iface.addr.ip().clone().to_string());
           }
        }
        
        if  broadcast_address.is_none() {
            println!("broadcast address was not found to have typical address pattern, \nlook into IP address configuration and enter your PC's local IP\n Enter here: ");
            let mut temp = String::new();
            std::io::stdin().read_line(&mut temp).unwrap();
            broadcast_address = Some(temp);
        }   

        let broadcast_address = broadcast_address.unwrap();

        let mut counter = 49153;
        while if_port_available(broadcast_address.clone(), counter) {
            counter += 1;
        }

        let client_host_port = counter;
        
        let broadcast_message = broadcast_address.clone();

        tokio::spawn(async move {
            Dominator::start_broadcast(format!("TS {}:{}",broadcast_message, client_host_port), argument2).await;
        });

        // now initializing all values has been done we can start the process of device assigning.
        let mut new = Dominator {clients, server_address, client_host_port, broadcast_address};
        new.connection_handler( client_host_port);
        return new;
    }

    //function that setups up a handler loop that accepts incoming connections then diverts them to
    //another assigned port by sending them the new IP address?.
    fn connection_handler(& mut self, port: i32) {
        TcpListener::bind(format!("{}:{}", self.broadcast_address , port)).unwrap();
        
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


fn if_port_available(address : String, port: i32) -> bool {
    TcpListener::bind(format!("{}:{}", address, port)).is_err()
}