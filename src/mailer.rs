use lettre::transport::smtp::{authentication::Credentials, PoolConfig};
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use std::string::String;


pub fn create_mailer(
    username: String,
    password: String,
    host: String,
) -> AsyncSmtpTransport<Tokio1Executor> {
    let creds = Credentials::new(username, password);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
        .unwrap()
        .credentials(creds)
        .pool_config(PoolConfig::new())
        .build();
    mailer
}