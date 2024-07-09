use std::{collections::HashMap, string, sync::{Arc, RwLock}};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use rand::{random, Rng};
use regex::Regex;
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;
use tokio_tungstenite::WebSocketStream;
use validator::{ValidateEmail, ValidationError};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::stream::SplitStream;
use lettre::{message::header::{ContentType, Date}, transport::smtp::commands::Auth};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message as mailMessage, SmtpTransport, Transport};
use chrono::{ DateTime, Days, Duration, Utc};
use tokio::sync::Mutex;
use serde_json::json;

use serde::{Deserialize, Serialize, Serializer,};
use serde_json::{Result, Value};


const TOKENLENGTH : i32 = 10;

#[derive(Serialize, Deserialize, Debug)]
#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Token {
    user: String,
    token_id: String,
    expire_date: DateTime<Utc>,
}



#[derive(Clone, Debug)]
pub struct User {
    email: String,
    password: String,
    name: String,
    active_tokens: Vec<Token>,
}

pub type Authenticator = Arc<Authenticate>;
pub struct Authenticate {
    total_logins: i32,
    user_data_base: Arc<Mutex<Vec<Mutex<User>>>>, //username and passwords.
    session_tokens: Mutex<HashMap<String, Token>>, //hashmap of session token -> user
    mailer: SmtpTransport,
    server_username: String,
}   

impl Authenticate {
    pub fn new() -> Arc<Self> {
        let credentials = include_bytes!("../../credentials.txt");
        let credentials = std::str::from_utf8(credentials).unwrap().to_string();
        let mut password = String::new();
        let mut username = String::new();
        let mut count = 0;
        for line in credentials.split("\n") {
            if count == 0{
                username += line;
            } else {
                password += line;
            }
            count +=1 ;
        }

        

        let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(Credentials::new(username.clone(), password))
        .build();

        let auth_obj = Arc::new(Authenticate {total_logins: 0, user_data_base: Arc::new(Mutex::new(Vec::new())), session_tokens: Mutex::new(HashMap::new()), mailer, server_username: username});
        
        let expire_thread = auth_obj.clone();
        tokio::spawn(async move {
            expire_thread.clear_expired_tokens().await;
        });

        auth_obj
    }

    /*
    * registers user and returns a token valid for 24 hours.
    */
    pub async fn register_user(&self, full_name: String, password: String, email: String, ws_sender: &mut SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>, ws_receiver: &mut SplitStream<WebSocketStream<TlsStream<TcpStream>>>) -> std::result::Result<Token,String> {
        //password checks
        let digit_regex = Regex::new(r"\d").unwrap();
        let uppercase_regex = Regex::new(r"[A-Z]").unwrap();
        if !(password.len() >= 10 && digit_regex.is_match(&password) && uppercase_regex.is_match(&password)){
            return Err("password does not satisfy".to_string());
        }

        //check if email is valid.
        if !validator::ValidateEmail::validate_email(&email){
            return Err(format!("{email} does not satisfy being an email."));
        }

        for user in self.user_data_base.lock().await.iter(){
            if user.lock().await.get_user_id() == email{
                ws_sender.send(Message::Text("{\"register\":\"failed, email is already used\"}".to_string())).await.unwrap();
                return Err(format!("{email} is already used"))
            }
        }

        let verification_code = rand::thread_rng().gen_range(0..999999);


        // let mail = mailMessage::builder()
        //     .from(format!("Trade Server <{}>",self.server_username.trim()).parse().unwrap())
        //     .to(email.parse().unwrap())
        //     .subject("Authentication Code For Trade Server")
        //     .header(ContentType::TEXT_PLAIN)
        //     .body(format!("Authentication code is {}",verification_code.clone()))
        //     .unwrap();

        // self.mailer.send(&mail).unwrap();
        
        // ws_sender.send(Message::text("{request: veri_code}")).await.unwrap(); //send a verification code request.
        // let mut tries = 0;
        // while let Some(msg) = ws_receiver.next().await {
        //     if tries == 6 {
        //         return Err("too many incorrect email verification tries".to_string());
        //     }
        //     let response = msg.unwrap().to_string();
        //     if response.contains("auth") {
        //         let response : Value = serde_json::from_str(&response).unwrap();
        //         let given_code = response.get("auth").unwrap().as_i64().unwrap();
        //         if  given_code == verification_code {
        //             break;
        //         } else {
        //             ws_sender.send(Message::text("{request: bad_veri_code}")).await.unwrap();
        //             tries += 1;
        //         }
        //     }
        // }

        //verification successful create user then create token.
        let mut new_user = User {email: email.clone(), password: password.clone(), name: full_name, active_tokens: Vec::new()};
        let token = self.create_token(&mut new_user).await;

        let mut data_bas = self.user_data_base.lock().await;
        let lock_user = Mutex::new(new_user);
        data_bas.push(lock_user);
        drop(data_bas);
        let token_json = serde_json::to_string(&token).unwrap();
        let wrapped_json = json!({ "token": token });
        let wrapped_json_string = serde_json::to_string(&wrapped_json).expect("Failed to serialize wrapped JSON");
        ws_sender.send(Message::Text(wrapped_json_string)).await.unwrap();
        Ok(token)
    }

    /*
    * logs in user and returns a token valid for 24 hours.
    */
    pub async fn login_user(&self, email: String, password: String, ws_sender: &mut SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>) -> std::result::Result<Token,String> {
        let lock = self.user_data_base.lock().await;
        let mut result: Option<User> = None;
        for user_lock in lock.iter(){
            let userr = user_lock.lock().await;
            if userr.get_user_id() == email && userr.get_pass_id() == password {
                result = Some(userr.clone());
            } 
        };  
        if result.is_some() {
            let token = self.create_token(&mut result.unwrap()).await;
            ws_sender.send(Message::Text(format!("{{\"login\": {{\"token\": {{\"user\": \"{}\", token_id: \"{}\", \"expire_date\": \"{}\"}}}}}}", token.user, token.token_id, token.expire_date))).await.unwrap();
            Ok(token)
        } else {
            ws_sender.send(Message::text("{login: {failed}}")).await.unwrap();
            return Err("authentication failed".to_string());
        }
    }

    /*
    * finds token and logs out users token.
    */
    pub async fn logout_user(&self, auth_token: Token) -> std::result::Result<(),String>{
        let lock = self.user_data_base.lock().await;
        for user_lock in lock.iter(){
            let mut  userr = user_lock.lock().await;
            if userr.active_tokens.contains(&auth_token){
                userr.active_tokens.retain(|x| *x != auth_token);
                println!("found token");
                let rock = self.session_tokens.lock().await.remove(&auth_token.get_id());
            } 
        };  
        Err("token not found".to_string())
    }

    pub async fn print_all_users(&self){
        let lock = self.user_data_base.lock().await;
        println!("all users: ");
        for user in lock.iter(){
            let lock = user.lock().await;
            println!("{:?}", lock);
        }
    }

    async fn clear_expired_tokens(self: Arc<Self>){
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(100)).await;
            let mut write_lock = self.session_tokens.lock().await;
        
            let expired_keys: Vec<String> = write_lock.iter()
            .filter(|(_, token)| token.get_expire() < Utc::now())
            .map(|(string_id, _)| string_id.clone())
            .collect();

            for string_id in expired_keys {
                let token = write_lock.remove(&string_id).unwrap();
                for user_lock in self.user_data_base.lock().await.iter(){
                    let mut  userr = user_lock.lock().await;
                    if userr.active_tokens.contains(&token){
                        userr.active_tokens.retain(|x| *x != token);
                        println!("found token");
                    } 
                }; 
            }
            
            self.print_all_users().await;

        }
    }

    /*
    * creates token
    */
    async fn create_token(&self, user: &mut User) -> Token {
        
        let mut random_string = generate_rand();
        let lock = self.session_tokens.lock().await;
        while lock.contains_key(&random_string) {
            random_string = generate_rand();
        }

        drop(lock);
        let mut expire_date = Utc::now();
        expire_date = expire_date.checked_add_days(Days::new(1)).unwrap();

        let new_token = Token {user: user.get_user_id().clone(), token_id: random_string.clone(), expire_date};
        user.add_token(new_token.clone());

        let mut lock = self.session_tokens.lock().await;
        lock.insert(random_string, new_token.clone());

        return new_token;
    }

}

fn generate_rand() -> String {
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
    abcdefghijklmnopqrstuvwxyz\
    0123456789";
    let mut rng = rand::thread_rng();
    let random_string: String = (0..TOKENLENGTH)
    .map(|_| {
        let idx = rng.gen_range(0..charset.len());
        charset[idx] as char
    })
    .collect();
    
    random_string
}


impl User {
    pub fn new(email: String, name: String, password: String) -> Self {
        Self {email, name, password, active_tokens: Vec::new()}
    }

    pub fn add_token(&mut self, token: Token) {
        self.active_tokens.push(token);
    }   

    pub fn get_user_id(&self) -> String {
        self.email.clone()
    }

    pub fn get_pass_id(&self) -> String {
        self.password.clone()
    }
}

impl Token {
    pub fn new(user: String, token_id: String, expire_date: DateTime<Utc>) -> Self {
        Self {user, token_id, expire_date}
    }
    pub fn get_user_id(&self) -> String{
        self.user.clone()
    }

    pub fn get_expire(&self) -> DateTime<Utc>{
        self.expire_date.clone()
    }

    pub fn get_id(&self) -> String {
        self.token_id.clone()
    }
}