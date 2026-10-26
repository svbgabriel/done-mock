use crate::model::FeedEntry;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

const BUFFER_CAPACITY: usize = 100;
const CHANNEL_CAPACITY: usize = 200;

#[derive(Clone)]
pub struct LiveFeed {
    history: Arc<RwLock<VecDeque<FeedEntry>>>,
    tx: broadcast::Sender<FeedEntry>,
}

impl Default for LiveFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveFeed {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(BUFFER_CAPACITY))),
            tx,
        }
    }

    pub fn record(&self, entry: FeedEntry) {
        {
            let mut guard = self.history.write().unwrap();
            if guard.len() >= BUFFER_CAPACITY {
                guard.pop_front();
            }
            guard.push_back(entry.clone());
        }
        let _ = self.tx.send(entry);
    }

    pub fn get_history(&self) -> Vec<FeedEntry> {
        let guard = self.history.read().unwrap();
        guard.iter().cloned().collect()
    }

    pub fn subscribe(&self) -> (Vec<FeedEntry>, broadcast::Receiver<FeedEntry>) {
        let history = self.get_history();
        let rx = self.tx.subscribe();
        (history, rx)
    }
}
