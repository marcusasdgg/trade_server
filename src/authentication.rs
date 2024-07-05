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
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message as mailMessage, SmtpTransport, Transport};
use chrono::{Date, DateTime, Days, Utc};
use tokio::sync::Mutex;

const TOKENLENGTH : i32 = 10;

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Token {
    user: String,
    token_id: String,
    expire_date: DateTime<Utc>,
}

pub struct User {
    email: String,
    password: String,
    name: String,
    active_tokens: Vec<Token>,
}
pub struct Authenticate {
    total_logins: i32,
    user_data_base: Arc<Mutex<Vec<User>>>, //username and passwords.
    session_tokens: RwLock<HashMap<String, Token>>, //hashmap of session token -> user
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
        Arc::new(Authenticate {total_logins: 0, user_data_base: Arc::new(Mutex::new(Vec::new())), session_tokens: RwLock::new(HashMap::new()), mailer, server_username: username})
    }

    /*
    * registers user and returns a token valid for 24 hours.
    */
    pub async fn register_user(&self, full_name: String, password: String, email: String, ws_sender: &mut SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>, ws_receiver: &mut SplitStream<WebSocketStream<TlsStream<TcpStream>>>) -> Result<(),String> {
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

        let verification_code = rand::thread_rng().gen_range(0..999999);

        let mail = mailMessage::builder()
            .from(self.server_username.clone().parse().unwrap())
            .to(email.parse().unwrap())
            .subject("Authentication Code For Trade Server")
            .header(ContentType::TEXT_PLAIN)
            .body(format!("Authentication code is {}",verification_code.clone()))
            .unwrap();

        self.mailer.send(&mail).unwrap();
        
        ws_sender.send(Message::text("{request: veri_code}")).await.unwrap(); //send a verification code request.
        let mut tries = 0;
        while let Some(msg) = ws_receiver.next().await {
            if tries == 6 {
                return Err("too many incorrect email verification tries".to_string());
            }
            let mut response = msg.unwrap().to_string();
            if response.contains("{auth: ") {
                if response.split_off(6).strip_suffix("}").unwrap().parse::<i32>().unwrap() == verification_code {
                    break;
                } else {
                    ws_sender.send(Message::text("{request: bad_veri_code}")).await.unwrap();
                    tries += 1;
                }
            }
        }

        //verification successful create user then create token.
        let mut new_user = User {email: email.clone(), password: password.clone(), name: full_name, active_tokens: Vec::new()};
        let token = self.create_token(&mut new_user);

        let mut data_bas = self.user_data_base.lock().await;
        data_bas.push(new_user);
        drop(data_bas);

        ws_sender.send(Message::Text(format!("{{token: {{user: {}, token_id: {}, expire_date: {}}}}}", token.user, token.token_id, token.expire_date))).await.unwrap();
        
        
        Ok(())
    }

    /*
    * logs in user and returns a token valid for 24 hours.
    */
    pub fn login_user() -> Result<String,String> {
        Ok("hash".to_string())
    }

    /*
    * creates token
    */
    fn create_token(&self, user: &mut User) -> Token {
        
        let mut random_string = generate_rand();
        let lock = self.session_tokens.read().unwrap();
        while lock.contains_key(&random_string) {
            random_string = generate_rand();
        }

        drop(lock);
        let mut expire_date = Utc::now();
        expire_date = expire_date.checked_add_days(Days::new(1)).unwrap();

        let new_token = Token {user: user.get_user_id().clone(), token_id: random_string, expire_date};
        user.add_token(new_token.clone());

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
}