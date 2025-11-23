use redis::{AsyncCommands, Client, aio::ConnectionManager};
use serde::Serialize;
use tracing::{debug, info};

/// Redis client for publishing events to Pub/Sub
#[derive(Clone)]
pub struct RedisClient {
    manager: ConnectionManager,
}

impl RedisClient {
    /// Create a new Redis client connection
    pub async fn new(redis_url: &str) -> color_eyre::Result<Self> {
        info!("Connecting to Redis at {}", redis_url);
        let client = Client::open(redis_url)?;
        let manager = client.get_connection_manager().await?;
        info!("Redis connection established successfully");
        Ok(Self { manager })
    }

    /// Publish a message to a Redis Pub/Sub channel
    pub async fn publish<T: Serialize>(
        &self,
        channel: &str,
        message: &T,
    ) -> color_eyre::Result<()> {
        let payload = serde_json::to_string(message)?;
        debug!("Publishing to channel '{}': {}", channel, payload);

        let mut conn = self.manager.clone();
        let subscribers: i32 = conn.publish(channel, payload).await?;

        debug!("Message published to {} subscribers", subscribers);
        metrics::counter!("redis_messages_published_total", "channel" => channel.to_string())
            .increment(1);

        Ok(())
    }

    /// Health check - ping Redis
    pub async fn ping(&self) -> color_eyre::Result<()> {
        let mut conn = self.manager.clone();
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        if pong != "PONG" {
            return Err(color_eyre::eyre::eyre!("Redis ping failed"));
        }
        debug!("Redis PING successful");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestMessage {
        id: String,
        content: String,
    }

    #[tokio::test]
    async fn test_redis_connection() {
        let redis_url = "redis://localhost:6380";
        let client = RedisClient::new(redis_url).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_redis_ping() {
        let redis_url = "redis://localhost:6380";
        let client = RedisClient::new(redis_url).await.unwrap();
        let result = client.ping().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_message() {
        let redis_url = "redis://localhost:6380";
        let client = RedisClient::new(redis_url).await.unwrap();

        let test_msg = TestMessage {
            id: "test-123".to_string(),
            content: "Hello Redis!".to_string(),
        };

        let result = client.publish("test_channel", &test_msg).await;
        assert!(result.is_ok());
    }
}
