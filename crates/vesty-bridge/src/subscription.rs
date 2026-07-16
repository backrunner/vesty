use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct SubscriptionTable {
    topics: BTreeSet<String>,
}

impl SubscriptionTable {
    pub fn subscribe(&mut self, topic: impl Into<String>) {
        self.topics.insert(topic.into());
    }

    pub fn unsubscribe(&mut self, topic: &str) {
        self.topics.remove(topic);
    }

    pub fn contains(&self, topic: &str) -> bool {
        self.topics.contains(topic)
    }

    pub fn len(&self) -> usize {
        self.topics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.topics.is_empty()
    }

    pub fn topics(&self) -> Vec<String> {
        self.topics.iter().cloned().collect()
    }
}
