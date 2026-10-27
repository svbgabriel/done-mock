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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FeedEntry;

    fn make_entry(id: &str) -> FeedEntry {
        FeedEntry {
            id: id.to_string(),
            timestamp: "2026-01-01T00:00:00.000Z".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            matched_mock_id: None,
            matched_mock_name: None,
            status: 200,
            duration_ms: 1.0,
        }
    }

    #[test]
    fn test_live_feed_new() {
        let feed = LiveFeed::new();
        assert_eq!(feed.get_history().len(), 0);
    }

    #[test]
    fn test_live_feed_default() {
        let feed = LiveFeed::default();
        assert_eq!(feed.get_history().len(), 0);
    }

    #[test]
    fn test_live_feed_record_and_history() {
        let feed = LiveFeed::new();
        assert_eq!(feed.get_history().len(), 0);

        feed.record(make_entry("1"));
        assert_eq!(feed.get_history().len(), 1);
        assert_eq!(feed.get_history()[0].id, "1");

        feed.record(make_entry("2"));
        assert_eq!(feed.get_history().len(), 2);
        assert_eq!(feed.get_history()[0].id, "1");
        assert_eq!(feed.get_history()[1].id, "2");
    }

    #[test]
    fn test_live_feed_buffer_capacity() {
        let feed = LiveFeed::new();
        // Record more entries than buffer capacity
        for i in 0..BUFFER_CAPACITY + 10 {
            feed.record(make_entry(&i.to_string()));
        }
        let history = feed.get_history();
        assert_eq!(history.len(), BUFFER_CAPACITY);
        // Oldest entries should be evicted
        assert_eq!(history[0].id, "10");
        assert_eq!(history[BUFFER_CAPACITY - 1].id, "109");
    }

    #[test]
    fn test_live_feed_subscribe_returns_history() {
        let feed = LiveFeed::new();
        feed.record(make_entry("1"));
        feed.record(make_entry("2"));

        let (history, _rx) = feed.subscribe();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, "1");
        assert_eq!(history[1].id, "2");
    }

    #[test]
    fn test_live_feed_subscribe_empty() {
        let feed = LiveFeed::new();
        let (history, _rx) = feed.subscribe();
        assert!(history.is_empty());
    }

    #[test]
    fn test_live_feed_clone() {
        let feed = LiveFeed::new();
        feed.record(make_entry("1"));

        let feed2 = feed.clone();
        assert_eq!(feed2.get_history().len(), 1);
        assert_eq!(feed2.get_history()[0].id, "1");
    }

    #[tokio::test]
    async fn test_live_feed_broadcast() {
        let feed = LiveFeed::new();
        let (_history, mut rx) = feed.subscribe();

        feed.record(make_entry("broadcast-test"));

        tokio::select! {
            res = rx.recv() => {
                assert!(res.is_ok());
                assert_eq!(res.unwrap().id, "broadcast-test");
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                panic!("Timeout waiting for broadcast");
            }
        }
    }
}
