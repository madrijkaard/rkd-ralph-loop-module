use config::{Config, File};

pub struct Settings {
    pub database_url: String,
}

pub fn load_config() -> Settings {
    let cfg = Config::builder()
        .add_source(File::with_name("config/default"))
        .build()
        .unwrap();

    Settings {
        database_url: cfg.get_string("database_url").unwrap(),
    }
}