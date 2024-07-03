use std::{collections::HashMap, string, sync::{Arc, Mutex, RwLock}};
use rand::{random, Rng};

const TOKENLENGTH : i32 = 10;

#[derive(Eq, Hash, PartialEq)]
pub struct Token {
    user: String,
    hash: String,
    expire_date: u64,
}
pub struct Authenticate {
    total_logins: i32,
    user_data_base: RwLock<Vec<(String, String, i32)>>, //username and passwords.
    session_tokens: RwLock<HashMap<String, Token>>, //hashmap of session token -> user
}

impl Authenticate {
    fn new() -> Arc<Self> {
        Arc::new(Authenticate {total_logins: 0, user_data_base: RwLock::new(Vec::new()), session_tokens: RwLock::new(HashMap::new())})
    }

    /*
    * registers user and returns a token valid for 24 hours.
    */
    pub fn register_user(&self) -> Result<String,String> {

        Ok(self.create_token())
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
    fn create_token(&self) -> String {
        
        let mut random_string = generate_rand();
        let lock = self.session_tokens.read().unwrap();
        while lock.contains_key(&random_string) {
            random_string = generate_rand();
        }
        random_string
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