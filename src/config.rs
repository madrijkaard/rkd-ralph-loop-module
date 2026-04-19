use config::{Config, File};

#[derive(Clone)]
pub struct Settings {
    pub database_url: String,
    pub engine_base_url: String,
}

pub fn load_config() -> Settings {
    let cfg = Config::builder()
        .add_source(File::with_name("config/default"))
        .build()
        .expect("Failed to load configuration");

    Settings {
        database_url: cfg
            .get_string("database_url")
            .expect("database_url not found in config"),

        engine_base_url: cfg
            .get_string("engine_base_url")
            .expect("engine_base_url not found in config"),
    }
}