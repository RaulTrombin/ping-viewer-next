#[macro_use]
extern crate lazy_static;

mod cli;
mod device;
mod logger;
mod server;

#[tokio::main]
async fn main() {
    // CLI should be started before logger to allow control over verbosity
    cli::manager::init();
    // Logger should start before everything else to register any log information
    logger::manager::init();

    // device manager todo as a service

    // server
    server::manager::run(&cli::manager::server_address())
        .await
        .unwrap();
}
