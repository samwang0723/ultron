use anyhow::{anyhow, Result};
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tokio::time::sleep;

pub struct Producer {
    producer: FutureProducer,
}

impl Producer {
    pub fn new(brokers: &str) -> Result<Self> {
        println!("Creating producer with brokers: {}", brokers);
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| anyhow!("Producer creation error: {}", e))?;
        Ok(Producer { producer })
    }

    pub async fn send(&self, topic: String, message: String) -> Result<(), anyhow::Error> {
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            let record: FutureRecord<String, String> = FutureRecord::to(&topic).payload(&message);
            let delivery_status = self.producer.send(record, Duration::from_secs(0)).await;
            match delivery_status {
                Ok(_) => return Ok(()),
                Err((kafka_error, _)) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow!(
                            "Failed to send message after {} attempts: {}",
                            attempts,
                            kafka_error
                        ));
                    }
                    println!(
                        "Failed to send message: {}. Retrying... (attempt {}/{})",
                        kafka_error, attempts, max_attempts
                    );
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}
