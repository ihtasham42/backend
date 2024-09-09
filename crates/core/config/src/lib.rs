use std::collections::HashMap;

use cached::proc_macro::cached;
use config::{Config, File, FileFormat};
use futures_locks::RwLock;
use once_cell::sync::Lazy;
use serde::Deserialize;

pub use sentry::capture_error;

/// Paths to search for configuration
static CONFIG_SEARCH_PATHS: [&str; 3] = [
    // current working directory
    "Revolt.toml",
    // current working directory - overrides file
    "Revolt.overrides.toml",
    // root directory, for Docker containers
    "/Revolt.toml",
];

/// Configuration builder
static CONFIG_BUILDER: Lazy<RwLock<Config>> = Lazy::new(|| {
    RwLock::new({
        let mut builder = Config::builder().add_source(File::from_str(
            include_str!("../Revolt.toml"),
            FileFormat::Toml,
        ));

        if std::env::var("TEST_DB").is_ok() {
            builder = builder.add_source(File::from_str(
                include_str!("../Revolt.test.toml"),
                FileFormat::Toml,
            ));
        }

        for path in CONFIG_SEARCH_PATHS {
            if std::path::Path::new(path).exists() {
                builder = builder.add_source(File::new(path, FileFormat::Toml));
            }
        }

        builder.build().unwrap()
    })
});

#[derive(Deserialize, Debug, Clone)]
pub struct Database {
    pub mongodb: String,
    pub redis: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Hosts {
    pub app: String,
    pub api: String,
    pub events: String,
    pub autumn: String,
    pub january: String,
    pub voso_legacy: String,
    pub voso_legacy_ws: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiRegistration {
    pub invite_only: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiSmtp {
    pub host: String,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub reply_to: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiVapid {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiFcm {
    pub key_type: String,
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_x509_cert_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiApn {
    pub sandbox: bool,
    pub pkcs8: String,
    pub key_id: String,
    pub team_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiSecurityCaptcha {
    pub hcaptcha_key: String,
    pub hcaptcha_sitekey: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiSecurity {
    pub authifier_shield_key: String,
    pub voso_legacy_token: String,
    pub captcha: ApiSecurityCaptcha,
    pub trust_cloudflare: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiWorkers {
    pub max_concurrent_connections: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Api {
    pub registration: ApiRegistration,
    pub smtp: ApiSmtp,
    pub vapid: ApiVapid,
    pub fcm: ApiFcm,
    pub apn: ApiApn,
    pub security: ApiSecurity,
    pub workers: ApiWorkers,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FilesLimit {
    pub min_resolution: [usize; 2],
    pub max_mega_pixels: usize,
    pub max_pixel_side: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FilesS3 {
    pub endpoint: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub default_bucket: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Files {
    pub encryption_key: String,
    pub webp_quality: f32,
    pub blocked_mime_types: Vec<String>,
    pub clamd_host: String,

    pub limit: FilesLimit,
    pub preview: HashMap<String, [usize; 2]>,
    pub s3: FilesS3,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GlobalLimits {
    pub group_size: usize,
    pub message_embeds: usize,
    pub message_replies: usize,
    pub message_reactions: usize,
    pub server_emoji: usize,
    pub server_roles: usize,
    pub server_channels: usize,

    pub new_user_days: usize,

    pub body_limit_size: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FeaturesLimits {
    pub outgoing_friend_requests: usize,

    pub bots: usize,
    pub message_length: usize,
    pub message_attachments: usize,
    pub servers: usize,

    pub attachment_size: usize,
    pub avatar_size: usize,
    pub background_size: usize,
    pub icon_size: usize,
    pub banner_size: usize,
    pub emoji_size: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FeaturesLimitsCollection {
    pub global: GlobalLimits,

    pub new_user: FeaturesLimits,
    pub default: FeaturesLimits,

    #[serde(flatten)]
    pub roles: HashMap<String, FeaturesLimits>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Features {
    pub limits: FeaturesLimitsCollection,
    pub webhooks_enabled: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Sentry {
    pub api: String,
    pub events: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub database: Database,
    pub hosts: Hosts,
    pub api: Api,
    pub files: Files,
    pub features: Features,
    pub sentry: Sentry,
}

impl Settings {
    pub fn preflight_checks(&self) {
        if self.api.smtp.host.is_empty() {
            log::warn!("No SMTP settings specified! Remember to configure email.");
        }

        if self.api.security.captcha.hcaptcha_key.is_empty() {
            log::warn!("No Captcha key specified! Remember to add hCaptcha key.");
        }
    }
}

pub async fn init() {
    println!(
        ":: Revolt Configuration ::\n\x1b[32m{:?}\x1b[0m",
        config().await
    );
}

pub async fn read() -> Config {
    CONFIG_BUILDER.read().await.clone()
}

#[cached(time = 30)]
pub async fn config() -> Settings {
    read().await.try_deserialize::<Settings>().unwrap()
}

/// Configure logging and common Rust variables
pub async fn setup_logging(release: &'static str, dsn: String) -> Option<sentry::ClientInitGuard> {
    dotenv::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    if std::env::var("ROCKET_ADDRESS").is_err() {
        std::env::set_var("ROCKET_ADDRESS", "0.0.0.0");
    }

    if std::env::var("REDIS_URL").is_err() {
        // Configure redis-kiss library
        let config = config().await;
        std::env::set_var("REDIS_URI", config.database.redis);
    }

    pretty_env_logger::init();
    log::info!("Starting {release}");

    if dsn.is_empty() {
        None
    } else {
        Some(sentry::init((
            dsn,
            sentry::ClientOptions {
                release: Some(release.into()),
                ..Default::default()
            },
        )))
    }
}

#[macro_export]
macro_rules! configure {
    ($application: ident) => {
        let config = $crate::config().await;
        let _sentry = $crate::setup_logging(
            concat!(env!("CARGO_PKG_NAME"), "@", env!("CARGO_PKG_VERSION")),
            config.sentry.$application,
        )
        .await;
    };
}

#[cfg(feature = "test")]
#[cfg(test)]
mod tests {
    use crate::init;

    #[async_std::test]
    async fn it_works() {
        init().await;
    }
}
