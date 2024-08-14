use deploy_rs::{deploy, init_config};
use tracing_subscriber::FmtSubscriber;

mod data;

fn main() {
    let subscriber = FmtSubscriber::builder().finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
    let deploy_config = init_config()
        .expect("Failed to init deploy config");
    deploy(deploy_config)
        .expect("Failed to deploy");
}
