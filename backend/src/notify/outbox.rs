//! A channel's outgoing queue: priority order, a size limit and a token bucket that keeps us
//! under Discord's rate limit (a webhook accepts about 30 messages a minute). Pure and driven by
//! an injected clock, so it is easy to test.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::Priority;

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub priority: Priority,
    pub body: Value,
}

#[derive(Debug)]
pub struct Outbox {
    items: VecDeque<Item>,
    capacity: usize,
    tokens: f64,
    burst: f64,
    per_second: f64,
    last_refill: Option<DateTime<Utc>>,
    dropped: u64,
}

impl Outbox {
    /// `per_minute` messages sustained, up to `burst` at once, at most `capacity` waiting.
    pub fn new(per_minute: f64, burst: f64, capacity: usize) -> Self {
        Self {
            items: VecDeque::new(),
            capacity,
            tokens: burst,
            burst,
            per_second: per_minute / 60.0,
            last_refill: None,
            dropped: 0,
        }
    }

    /// Discord allows ~30 a minute per webhook: stay clearly under it.
    pub fn standard() -> Self {
        Self::new(20.0, 5.0, 50)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Notices thrown away because the queue was full.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Queues `item`. When full, the least important waiting notice (the oldest of its priority)
    /// makes room, unless the new one is the least important: then the new one is dropped.
    pub fn push(&mut self, item: Item) {
        if self.items.len() >= self.capacity {
            let worst = self
                .items
                .iter()
                .enumerate()
                .max_by_key(|(i, it)| (it.priority, std::cmp::Reverse(*i)))
                .map(|(i, it)| (i, it.priority));
            match worst {
                Some((i, p)) if p > item.priority => {
                    self.items.remove(i);
                    self.dropped += 1;
                }
                _ => {
                    self.dropped += 1;
                    return;
                }
            }
        }
        self.items.push_back(item);
    }

    /// The next message to send now, if the rate limit allows: most important first, in order of
    /// arrival among equals.
    pub fn pop(&mut self, now: DateTime<Utc>) -> Option<Item> {
        if let Some(last) = self.last_refill {
            let secs = (now - last).num_milliseconds().max(0) as f64 / 1000.0;
            self.tokens = (self.tokens + secs * self.per_second).min(self.burst);
        }
        self.last_refill = Some(now);
        if self.tokens < 1.0 {
            return None;
        }
        let best = self
            .items
            .iter()
            .enumerate()
            .min_by_key(|(i, it)| (it.priority, *i))
            .map(|(i, _)| i)?;
        self.tokens -= 1.0;
        self.items.remove(best)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn t(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_790_000_000 + secs, 0).unwrap()
    }
    fn item(p: Priority, n: u32) -> Item {
        Item {
            priority: p,
            body: json!({ "n": n }),
        }
    }
    fn n(i: Option<Item>) -> Option<u64> {
        i.and_then(|i| i.body["n"].as_u64())
    }

    #[test]
    fn important_first_and_in_order_among_equals() {
        let mut o = Outbox::new(60.0, 10.0, 10);
        for (p, k) in [
            (Priority::Low, 1),
            (Priority::Normal, 2),
            (Priority::High, 3),
            (Priority::Normal, 4),
            (Priority::High, 5),
        ] {
            o.push(item(p, k));
        }
        let order: Vec<_> = (0..5).map(|_| n(o.pop(t(0)))).collect();
        assert_eq!(order, [Some(3), Some(5), Some(2), Some(4), Some(1)]);
        assert!(o.pop(t(0)).is_none());
    }

    #[test]
    fn the_token_bucket_spreads_a_burst_under_the_limit() {
        // 20 a minute = one every 3 s, burst of 2.
        let mut o = Outbox::new(20.0, 2.0, 10);
        for k in 0..5 {
            o.push(item(Priority::Normal, k));
        }
        assert!(o.pop(t(0)).is_some());
        assert!(o.pop(t(0)).is_some());
        assert!(o.pop(t(0)).is_none(), "burst spent");
        assert!(o.pop(t(1)).is_none());
        assert!(o.pop(t(4)).is_some(), "one token came back");
        assert!(o.pop(t(4)).is_none());
        assert!(o.pop(t(60)).is_some());
        assert!(o.pop(t(60)).is_some());
        assert!(o.pop(t(60)).is_none(), "never more than the burst");
    }

    #[test]
    fn when_full_the_least_important_makes_room() {
        let mut o = Outbox::new(60.0, 10.0, 3);
        o.push(item(Priority::Low, 1));
        o.push(item(Priority::Normal, 2));
        o.push(item(Priority::Low, 3));
        o.push(item(Priority::High, 4)); // evicts the oldest Low (1)
        assert_eq!((o.len(), o.dropped()), (3, 1));
        o.push(item(Priority::Low, 5)); // not better than anything waiting: dropped itself
        assert_eq!((o.len(), o.dropped()), (3, 2));
        let order: Vec<_> = (0..3).map(|_| n(o.pop(t(0)))).collect();
        assert_eq!(order, [Some(4), Some(2), Some(3)]);
    }
}
