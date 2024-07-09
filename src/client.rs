use std::{borrow::Borrow, net::SocketAddr, str::FromStr, sync::Arc, string};

use chrono::{DateTime, Utc};
use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_native_tls::TlsStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::authentication::{Authenticate, Authenticator, Token};
use serde::Deserialize;
use serde_json::{Result, Value};
use futures_util::FutureExt;
pub struct Client {
    ip_address: SocketAddr,
    authenticator: Authenticator,
    session_token: String,
    //broadcast message channel.
}

impl Client {
    pub fn new(ws_sender: SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>,ws_receiver: SplitStream<WebSocketStream<TlsStream<TcpStream>>>, authenticator: Authenticator, adress: SocketAddr, token: String) -> (Arc<Self>, tokio::task::JoinHandle<()>)  {
        //loop di doop inn 
        let client = Arc::new(Self { authenticator, ip_address: adress, session_token: token});
        let client_loop = client.clone();
        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            client_loop.handle_loop(ws_sender,ws_receiver).await;
        }); 
        (client, handle)
    }   

    async fn handle_loop(&self, mut ws_sender: SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>,mut ws_receiver: SplitStream<WebSocketStream<TlsStream<TcpStream>>>) {

        while let Some(msg) = ws_receiver.next().await {
            // all conditions
            if msg.is_err() {
                return;
            }
            let request: Value = serde_json::from_str(&msg.unwrap().to_string()).unwrap();

            //open up message channel

            if let Some(req) = request.get("logout"){
                let token = req.get("token").unwrap();
                let if_valid = serialize_to_token(token);
                if if_valid.is_none(){
                    ws_sender.send(Message::text("{logout: failed token unable to be serialized}".to_string())).await.unwrap();
                    continue;
                }
                if self.authenticator.logout_user(if_valid.unwrap()).await.is_err(){
                    ws_sender.send(Message::text("{logout: success}".to_string())).await.unwrap();
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
        }

    }


}

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