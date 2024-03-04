use lettre::message::header::ContentType;
use lettre::transport::smtp::{authentication::Credentials, PoolConfig};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::string::String;

pub struct Mailer {
    pub mailer: AsyncSmtpTransport<Tokio1Executor>,
}

pub fn create_mailer(
    username: String,
    password: String,
    host: String,
) -> AsyncSmtpTransport<Tokio1Executor> {
    let creds = Credentials::new(username, password);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
        .unwrap()
        .credentials(creds)
        .port(2525)
        .pool_config(PoolConfig::new())
        .build();
    mailer
}

impl Mailer {
    pub fn create_mailer(username: String, password: String, host: String) -> Mailer {
        let creds = Credentials::new(username, password);
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .unwrap()
            .credentials(creds)
            .pool_config(PoolConfig::new())
            .build();
        Mailer { mailer }
    }

    async fn _send_email(&self, message: Message) -> Result<(), String> {
        match self.mailer.send(message).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

pub async fn _send_test_email(mail_transport: &Mailer) {
    let email = Message::builder()
        .from("HackDuke <noreply@hackduke.org>".parse().unwrap())
        .to("James Xu <james@jamesxu.ca>".parse().unwrap())
        .subject("Happy new async year")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Be happy with async!"))
        .unwrap();

    match mail_transport._send_email(email).await {
        Ok(_) => log::info!("email sent"),
        Err(e) => log::error!("{}", e),
    }
}
