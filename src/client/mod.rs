use std::{borrow::Borrow, collections::HashMap, net::SocketAddr, ops::DerefMut, str::FromStr, string, sync::Arc, time::Duration};


use chrono::{DateTime, Utc};
use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use yahoo_tick_grabber::{fin_retypes::FinResult, YAHOOCONNECT};

use crate::{authentication::{Authenticate, Authenticator, Token}, watchlistmap::WatchListMap};
use serde::Deserialize;
use serde_json::{Result, Value};
use futures_util::FutureExt;



pub struct Client {
    ip_address: SocketAddr,
    authenticator: Authenticator,
    ws_receiver: Arc<Mutex<SplitStream<WebSocketStream<TlsStream<TcpStream>>>>>,
    ws_sender: Arc<Mutex<SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>>>,
    watchlist_store: Arc<WatchListMap>,
    
    subscribed_info: Mutex<HashMap<String, FinResult>>,
    // we have access to a reference to watchlist map, every second we call the function of all trades 
    // we store trades in an array, setup a subscriber loop.
    //broadcast message channel.
}

impl Client {
    pub fn new(ws_sender: SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>,ws_receiver: SplitStream<WebSocketStream<TlsStream<TcpStream>>>, authenticator: Authenticator, adress: SocketAddr, watchlist_store: Arc<WatchListMap>) -> (Arc<Self>, tokio::task::JoinHandle<()>)  {
        //loop di doop inn 
        let client = Arc::new(Self { authenticator, ip_address: adress, ws_receiver: Arc::new(Mutex::new(ws_receiver)), ws_sender: Arc::new(Mutex::new(ws_sender)), watchlist_store, subscribed_info: Mutex::new(HashMap::new())});
        let client_loop = client.clone();
        let sub_loop = client.clone();
        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            client_loop.handle_loop().await;
        });

        tokio::spawn(async move {
            sub_loop.subscribe_loop().await;
        });
        //new clone

        (client, handle)
    }   

    async fn handle_loop(&self) {
        let mut ws_receiver = self.ws_receiver.lock().await; //ws receiver should mostly only be used by handle loop.

        while let Some(msg) = ws_receiver.next().await {
            // all conditions
            if msg.is_err() {
                return;
            }
            let request: Value = serde_json::from_str(&msg.unwrap().to_string()).unwrap();

            //open up message channel

            if let Some(req) = request.get("logout"){
                let mut ws_sender = self.ws_sender.lock().await;
                let token = req.get("token").unwrap();
                let if_valid = serialize_to_token(token);
                if if_valid.is_none(){
                    ws_sender.send(Message::text("{\"logout\": \"failed token unable to be serialized\"}".to_string())).await.unwrap();
                    continue;
                }
                if self.authenticator.logout_user(if_valid.unwrap()).await.is_err(){
                    ws_sender.send(Message::text("{\"logout\": \"success\"}".to_string())).await.unwrap();
                    ws_sender.send(Message::Close(None)).await.unwrap();
                    while let Some(msg) = ws_receiver.next().await {
                        if msg.is_err() {
                            return;
                        }

                        if msg.unwrap().is_close() {
                            return;
                        }
                    }
                    return;
                }else {
                    println!("token aint valid");
                    ws_sender.send(Message::text("{logout: failed}".to_string())).await.unwrap();
                }
                continue;
            }

            if let Some(req) = request.get("watchlist_add") {
                
            }

            if let Some(req) = request.get("watchlist_remove") {
                
            }

            if let Some(req) = request.get("watchlist_start") {

            }

            if let Some(req) = request.get("watchlist_stop") {

            }

            if let Some(req) = request.get("trade_add") {

            }
        }

    }

    async fn subscribe_loop(&self){
        loop {
            let mut lock = self.subscribed_info.lock().await;
            for i in lock.iter_mut() {
                *i.1 = self.watchlist_store.get_value(i.0).await[0].clone();
            }
            //delay by 1000
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }


}

//how do we want to implement subcriber functionality

fn serialize_to_token(tokend: &Value) -> Option<Token> {
    println!("{tokend}");
    let user = tokend.get("user");
    let token_id = tokend.get("token_id");
    let expire_date = tokend.get("expire_date");
    if user.is_none() || token_id.is_none() || expire_date.is_none(){
        println!("failed");
        return None;
    }
    let user = user.unwrap().to_string().trim_matches('"').to_string();
    let token_id = token_id.unwrap().to_string().trim_matches('"').to_string();
    let expire_date: DateTime<Utc> = DateTime::parse_from_rfc3339(expire_date.unwrap().as_str().unwrap()).unwrap().to_utc();
    Some(Token::new(user, token_id, expire_date))
    
}