use config::Config;
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub kafka: KafkaSettings,
    pub proxy: ProxySettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: String,
    pub user: String,
    pub dbname: String,
    #[serde(skip_deserializing)]
    pub dbpasswd: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaSettings {
    pub brokers: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProxySettings {
    pub host: String,
    pub port: String,
    pub username: String,
    #[serde(skip_deserializing)]
    pub passwd: String,
}

lazy_static! {
    pub static ref SETTINGS: Settings = Settings::new();
}

impl Settings {
    pub fn new() -> Self {
        let env = std::env::var("ENV").unwrap();
        let mut cfg = Self::load_from_file(&env);
        Self::customize_from_env(&mut cfg);
        cfg
    }

    fn load_from_file(env: &str) -> Self {
        let file = format!("config.{}.yaml", env);
        let settings = Config::builder()
            .add_source(config::File::with_name(&file))
            .build()
            .expect("Failed to build configuration");

        settings
            .try_deserialize()
            .expect("Failed to deserialize configuration")
    }

    fn customize_from_env(cfg: &mut Self) {
        if let Ok(db_password) = std::env::var("DB_PASSWD") {
            cfg.database.dbpasswd = db_password;
        }

        if let Ok(proxy_password) = std::env::var("PROXY_PASSWD") {
            cfg.proxy.passwd = proxy_password;
        }
    }
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.dbpasswd, self.host, self.port, self.dbname
        )
    }
}
