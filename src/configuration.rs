use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
}

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub chain_fork_height: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub number_of_chains: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub limit: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub min_height: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub max_height: u64,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let config_dir = base_path.join("configuration");

    // Detect the running environment
    // Default to `local` if unspecified
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");
    let environment_filename = format!("{}.yaml", environment.as_str());

    // Read default configuration
    let settings = config::Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")).required(true))
        .add_source(config::File::from(config_dir.join(&environment_filename)).required(true))
        .build()?;
    settings.try_deserialize::<Settings>()
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}
