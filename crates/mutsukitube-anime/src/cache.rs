use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct CacheEntry<V> {
    pub value: V,
    pub expires_at: Instant,
}

#[derive(Debug)]
pub struct BoundedCache<K, V> {
    capacity: usize,
    ttl: Duration,
    entries: HashMap<K, CacheEntry<V>>,
    order: VecDeque<K>,
}

impl<K, V> BoundedCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            capacity: capacity.max(1),
            ttl,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&mut self, key: &K, now: Instant) -> Option<V> {
        if let Some(entry) = self.entries.get(key) {
            if now < entry.expires_at {
                return Some(entry.value.clone());
            }
        }
        self.entries.remove(key);
        self.order.retain(|k| k != key);
        None
    }

    pub fn insert(&mut self, key: K, value: V, now: Instant) {
        if self.entries.contains_key(&key) {
            self.entries.insert(
                key.clone(),
                CacheEntry {
                    value,
                    expires_at: now + self.ttl,
                },
            );
            return;
        }

        while self.entries.len() >= self.capacity {
            if let Some(old) = self.order.pop_front() {
                self.entries.remove(&old);
            } else {
                break;
            }
        }

        self.entries.insert(
            key.clone(),
            CacheEntry {
                value,
                expires_at: now + self.ttl,
            },
        );
        self.order.push_back(key);
    }

    pub fn invalidate(&mut self, key: &K) {
        self.entries.remove(key);
        self.order.retain(|k| k != key);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut cache = BoundedCache::new(2, Duration::from_secs(10));
        let now = Instant::now();
        cache.insert("a", 1, now);
        assert_eq!(cache.get(&"a", now), Some(1));
    }

    #[test]
    fn expiry() {
        let mut cache = BoundedCache::new(2, Duration::from_secs(10));
        let now = Instant::now();
        cache.insert("a", 1, now);
        assert_eq!(cache.get(&"a", now + Duration::from_secs(11)), None);
    }

    #[test]
    fn eviction() {
        let mut cache = BoundedCache::new(1, Duration::from_secs(10));
        let now = Instant::now();
        cache.insert("a", 1, now);
        cache.insert("b", 2, now);
        assert_eq!(cache.get(&"a", now), None);
        assert_eq!(cache.get(&"b", now), Some(2));
    }

    #[test]
    fn invalidate() {
        let mut cache = BoundedCache::new(2, Duration::from_secs(10));
        let now = Instant::now();
        cache.insert("a", 1, now);
        cache.invalidate(&"a");
        assert_eq!(cache.get(&"a", now), None);
    }
}
