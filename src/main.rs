mod api;
mod mailer;
use std::env;

use log;

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    println!("Hello, world!");
    let smtp_username = env::var("SMTP_USERNAME").unwrap();
    let smtp_password = env::var("SMTP_PASSWORD").unwrap();
    let smtp_host = env::var("SMTP_HOST").unwrap();
    log::info!("Loaded SMTP configuration");
    let smtp_mailer = mailer::Mailer::create_mailer(smtp_username, smtp_password, smtp_host);
}
