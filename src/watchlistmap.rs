use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{collections::HashMap, thread::sleep};
use tokio::sync::RwLock;
use tokio::time;
use yahoo_tick_grabber::{fin_retypes::FinResult, YAHOOCONNECT};

pub struct WatchListMap {
    api: YAHOOCONNECT,
    map: RwLock<HashMap<String, Vec<FinResult>>>,
}

impl WatchListMap {
    pub async fn new() -> Arc<Self> {
        let api = YAHOOCONNECT::new().await.unwrap();
        let map = tokio::sync::RwLock::new(HashMap::new());

        let ret = Arc::new(WatchListMap { api, map });

        ret
    }

    pub async fn get_value(self: &Arc<Self>, name: &str) -> Vec<FinResult> {
        let rawr = self.map.read().await;
        //println!("map right now is {:?}", rawr);
        if rawr.contains_key(name) {
            return self.get_cache(name).await.unwrap();
        } else {
            drop(rawr);
            println!("cache miss");

            let rt = self.api.get_ticker(name).await.unwrap();
            let delf = self.clone();
            delf.cache_val(name.to_string(), rt.clone()).await.unwrap();
            return rt;
        }
        //check cache first then send api request.
    }

    async fn cache_val(
        self: Arc<Self>,
        name: String,
        data: Vec<FinResult>,
    ) -> std::result::Result<(), String> {
        let mut chenis = self.map.write().await;
        chenis.insert(name.clone(), data);
        drop(chenis);
        let d_self = self.clone();
        tokio::spawn(async move {
            time::sleep(Duration::from_secs(3)).await;
            d_self.clear_cache_value(name).await;
        });
        return Ok(());
    }

    async fn get_cache(&self, name: &str) -> Option<Vec<FinResult>> {
        let read = self.map.read().await;
        let ret = read.get(name).cloned();
        drop(read);
        return ret;
    }

    async fn clear_cache_value(&self, name: String) {
        let mut lock = self.map.write().await;
        lock.remove(&name);
    }
}
