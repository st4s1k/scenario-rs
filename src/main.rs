use colored::*;
use ssh2::Session;
use std::process;
use tracing::error;
use deploy_rs::{
    deploy,
    init_config,
    init_session,
    init_tracing_subscriber
};

mod data;

const SEPARATOR: &str = "------------------------------------------------------------";

macro_rules! handle_error {
    ($message:expr, $error:expr) => {
        {
            error!("{}", SEPARATOR);
            error!("{}: {}", $message.bold(), $error);
            error!("{}", SEPARATOR);
            process::exit(1);
        }
    };
}

fn main() {
    init_tracing_subscriber();
    let config = match init_config() {
        Ok(config) => config,
        Err(error) => handle_error!("Init config failed", error)
    };
    let session: Session = match init_session(&config) {
        Ok(session) => session,
        Err(error) => handle_error!("Init session failed", error)
    };
    if let Err(error) = deploy(config, session) {
        handle_error!("Deploy failed", error)
    }
}
