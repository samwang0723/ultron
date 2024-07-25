use anyhow::anyhow;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

pub struct Producer {
    producer: FutureProducer,
}

impl Producer {
    pub fn new(brokers: &str) -> Self {
        println!("Creating producer with brokers: {}", brokers);
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer creation error");
        Producer { producer }
    }

    pub async fn send(&self, topic: String, message: String) -> Result<(), anyhow::Error> {
        let record: FutureRecord<String, String> = FutureRecord::to(&topic).payload(&message);
        let delivery_status = self.producer.send(record, Duration::from_secs(0)).await;
        match delivery_status {
            Ok(_) => Ok(()),
            Err((kafka_error, _)) => Err(anyhow!("Failed to send message: {}", kafka_error)),
        }
    }
}
