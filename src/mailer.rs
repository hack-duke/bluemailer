use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, Transport};
use std::string::String;

pub struct Mailer {
    creds: Credentials,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl Mailer {
    pub fn create_mailer(username: String, password: String, host: String) -> Mailer {
        let creds = Credentials::new(username, password);
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .unwrap()
            .credentials(creds.clone())
            .build();
        Mailer { creds, mailer }
    }

    pub async fn send_email(&self, message: Message) {
        match self.mailer.send(message).await {
            Ok(_) => {}
            Err(_) => {}
        }
    }
}
