//! Redis Pub/Sub for distributed cache invalidation
//!
//! Enables cache invalidation events to be broadcast across multiple
//! application instances sharing the same Redis backend.

use std::sync::Arc;
use tokio::sync::broadcast;

/// Channel name for cache invalidation events
pub const INVALIDATION_CHANNEL: &str = "skp_cache:invalidate";

/// Event types for distributed invalidation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationEvent {
    /// Single key invalidated
    Key(String),
    /// Multiple keys invalidated by pattern
    Pattern(String),
    /// Tag-based invalidation
    Tag(String),
    /// Full cache clear
    Clear,
}

impl InvalidationEvent {
    /// Serialize event to Redis message format
    pub fn to_message(&self) -> String {
        match self {
            InvalidationEvent::Key(k) => format!("key:{}", k),
            InvalidationEvent::Pattern(p) => format!("pattern:{}", p),
            InvalidationEvent::Tag(t) => format!("tag:{}", t),
            InvalidationEvent::Clear => "clear".to_string(),
        }
    }

    /// Parse event from Redis message
    pub fn from_message(msg: &str) -> Option<Self> {
        if msg == "clear" {
            return Some(InvalidationEvent::Clear);
        }

        let (prefix, value) = msg.split_once(':')?;
        let value = value.to_string();

        match prefix {
            "key" => Some(InvalidationEvent::Key(value)),
            "pattern" => Some(InvalidationEvent::Pattern(value)),
            "tag" => Some(InvalidationEvent::Tag(value)),
            _ => None,
        }
    }
}

/// Publisher for sending invalidation events
#[derive(Clone)]
pub struct InvalidationPublisher {
    tx: broadcast::Sender<InvalidationEvent>,
}

impl InvalidationPublisher {
    /// Create a new publisher
    pub fn new(capacity: usize) -> (Self, InvalidationSubscriber) {
        let (tx, rx) = broadcast::channel(capacity);
        (
            Self { tx },
            InvalidationSubscriber { rx },
        )
    }

    /// Publish an invalidation event
    pub fn publish(&self, event: InvalidationEvent) -> Result<usize, PublishError> {
        self.tx.send(event).map_err(|_| PublishError::NoSubscribers)
    }

    /// Get a new subscriber
    pub fn subscribe(&self) -> InvalidationSubscriber {
        InvalidationSubscriber {
            rx: self.tx.subscribe(),
        }
    }
}

/// Subscriber for receiving invalidation events
pub struct InvalidationSubscriber {
    rx: broadcast::Receiver<InvalidationEvent>,
}

impl InvalidationSubscriber {
    /// Receive the next invalidation event
    pub async fn recv(&mut self) -> Result<InvalidationEvent, SubscribeError> {
        self.rx.recv().await.map_err(|e| match e {
            broadcast::error::RecvError::Closed => SubscribeError::Closed,
            broadcast::error::RecvError::Lagged(n) => SubscribeError::Lagged(n),
        })
    }
}

/// Error when publishing events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishError {
    /// No subscribers listening
    NoSubscribers,
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::NoSubscribers => write!(f, "no subscribers listening"),
        }
    }
}

impl std::error::Error for PublishError {}

/// Error when subscribing to events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscribeError {
    /// Channel closed
    Closed,
    /// Subscriber lagged behind
    Lagged(u64),
}

impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscribeError::Closed => write!(f, "channel closed"),
            SubscribeError::Lagged(n) => write!(f, "lagged behind by {} messages", n),
        }
    }
}

impl std::error::Error for SubscribeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let events = vec![
            (InvalidationEvent::Key("foo".into()), "key:foo"),
            (InvalidationEvent::Pattern("user:*".into()), "pattern:user:*"),
            (InvalidationEvent::Tag("users".into()), "tag:users"),
            (InvalidationEvent::Clear, "clear"),
        ];

        for (event, expected) in events {
            let msg = event.to_message();
            assert_eq!(msg, expected);

            let parsed = InvalidationEvent::from_message(&msg);
            assert_eq!(parsed, Some(event));
        }
    }

    #[tokio::test]
    async fn test_pubsub() {
        let (publisher, mut subscriber) = InvalidationPublisher::new(16);

        // Publish
        publisher.publish(InvalidationEvent::Key("test".into())).unwrap();

        // Receive
        let event = subscriber.recv().await.unwrap();
        assert_eq!(event, InvalidationEvent::Key("test".into()));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let (publisher, mut sub1) = InvalidationPublisher::new(16);
        let mut sub2 = publisher.subscribe();

        publisher.publish(InvalidationEvent::Clear).unwrap();

        let e1 = sub1.recv().await.unwrap();
        let e2 = sub2.recv().await.unwrap();

        assert_eq!(e1, InvalidationEvent::Clear);
        assert_eq!(e2, InvalidationEvent::Clear);
    }
}
