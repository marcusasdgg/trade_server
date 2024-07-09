use std::{net::{SocketAddr, UdpSocket}, option, sync::{Arc, Mutex}};
use authentication::Authenticate;
use tokio::{net, sync::broadcast};
use tokio::spawn;
use tokio::time::Duration;
use if_addrs::get_if_addrs;
use tokio::net::{TcpListener, TcpStream};
use tokio_native_tls::TlsAcceptor;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use native_tls::{Identity, TlsAcceptor as NativeTlsAcceptor};
use futures_util::{StreamExt, SinkExt};


use serde::Deserialize;
use serde_json::{Result, Value};

mod client;
mod authentication;
use authentication::Token;
use client::Client;

pub struct Dominator {
    clients: Arc<Mutex<Vec<Arc<Client>>>>,
    server_address: String,
    client_host_port: i32,
    broadcast_address: String,
    authenticator: Arc<Authenticate>
    //atomic bool for shutdown sequence.
    //saved_information struct with yahooo connected
    // tradeApi
    // trade_info API.
}   

impl Dominator {
    pub async fn new() -> Arc<Self> //no checking for shitty address rn.
    {
        println!("starting server up!");
        //initialize dominator fields,
        let clients = Arc::new(Mutex::new(Vec::new()));

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
        while if_port_available(broadcast_address.clone(), counter).await {
            counter += 1;
        }

        let client_host_port = counter;
        
        let broadcast_message = broadcast_address.clone();

        tokio::spawn(async move {
            Dominator::start_broadcast(format!("TS {}:{}",broadcast_message, client_host_port), argument2).await;
        });

        // now initializing all values has been done we can start the process of device assigning.
        let new = Arc::new(Dominator {clients, server_address, client_host_port, broadcast_address, authenticator: Authenticate::new()});
        
        let handler_new = new.clone();
        tokio::spawn(async move {
           handler_new.connection_handler(client_host_port).await;
        });

        return new;
    }

    //function that setups up a handler loop that accepts incoming connections then diverts them to
    //another assigned port by sending them the new IP address?.
    async fn connection_handler(self: Arc<Self>, port: i32) {
        // bind to broadcast address and port. we then establish
        let listener = TcpListener::bind(format!("{}:{}", self.broadcast_address , port)).await.unwrap();
        let identity = load_tls_identity().expect("Failed to load TLS identity");
        
        let acceptor = TlsAcceptor::from(identity);
        //initate websocket oonnection
        while let Ok((stream, _)) = listener.accept().await {
            println!("found connection");
            let acceptor = acceptor.clone();
            let this = self.clone();
            tokio::spawn(async move {
                let tls_stream = acceptor.accept(stream).await.unwrap();
                this.handle_connection(tls_stream).await;
            });
        }
        println!("oh shit");
    }

    
    async fn handle_connection(&self,  stream: tokio_native_tls::TlsStream<tokio::net::TcpStream>){
        let peer_addr = stream.get_ref().get_ref().get_ref().peer_addr().unwrap();
        
        println!("address of client is {}", peer_addr);
        let mut logged_in: Option<Token> = None;
        let ws_stream = accept_async(stream).await.expect("Error during WebSocket handshake");
        println!("websocket found");
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        while let Some(msg) = ws_receiver.next().await {
            //we capture this stream, once the user is authenticated we send stream to a client object to handle.
            let msg = msg.unwrap().into_text().unwrap();
            if msg.contains("register"){
                println!("user trying to register");
                let d: Value = serde_json::from_str(&msg).unwrap();
                let s : &Value = d.get("register").unwrap();
                let email = s.get("email").unwrap().to_string().trim_matches('"').to_string();
                let password = s.get("password").unwrap().to_string().trim_matches('"').to_string();
                let full_name = s.get("name").unwrap().to_string().trim_matches('"').to_string();

                println!("found json: {}, {}, {}",email, password, full_name );
                let register = self.authenticator.register_user(full_name, password, email, &mut ws_sender, &mut ws_receiver).await;
                if register.is_ok(){
                    logged_in = Some(register.unwrap());
                    break;
                } else {
                    break;
                }
            } else if msg.contains("login"){
                println!("user trying to login");
                let d: Value = serde_json::from_str(&msg).unwrap();
                let s : &Value = d.get("login").unwrap();
                let email = s.get("email").unwrap().to_string().trim_matches('"').to_string();
                let password = s.get("password").unwrap().to_string().trim_matches('"').to_string();

                let login = self.authenticator.login_user(email, password, &mut ws_sender).await;
                if login.is_ok(){
                    let logged_in = Some(login.unwrap());
                    break;
                } else {
                    println!("authentication failed");
                }
            } 
            // we authenticate this dude first then hand it off to client.
        }

        //write create new client function, consuming ws_sender otherwise drop it.
        if logged_in.is_some() {
            let client = Client::new(ws_sender, ws_receiver, self.authenticator.clone(), peer_addr, logged_in.unwrap().get_id());
            self.clients.lock().unwrap().push(client.0);
            client.1.await.unwrap();
        } else {
            return;
        }
    }

    

    async fn start_broadcast(message: String, broadcast_address: String){

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


async fn if_port_available(address : String, port: i32) -> bool {
    TcpListener::bind(format!("{}:{}", address, port)).await.is_err()
}

fn load_tls_identity() -> std::result::Result<NativeTlsAcceptor, Box<dyn std::error::Error>> {
    // Load the identity from a PKCS#12 archive
    let cert = include_bytes!("identity.pfx");
    let identity = Identity::from_pkcs12(cert, "Yahooconnect!1")?;
    Ok(NativeTlsAcceptor::builder(identity).build()?)
}

