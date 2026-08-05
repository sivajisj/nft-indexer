use std::env;

pub struct Config {
    pub rpc_url: String,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Config {
            rpc_url: env::var("RPC_URL").expect("RPC_URL must be set"),
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        }
    }
}
