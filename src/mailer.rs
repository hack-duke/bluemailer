use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::error::Error;
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

    pub async fn send_email(&self, message: Message) -> Result<(), String>{
        match self.mailer.send(message).await {
            Ok(_) => {
                Ok(())
            }
            Err(e) => Err(e.to_string())
        }
    }
}

pub async fn send_test_email(mail_transport: &Mailer) {
    let email = Message::builder()
        .from("HackDuke <noreply@hackduke.org>".parse().unwrap())
        .to("James Xu <james@jamesxu.ca>".parse().unwrap())
        .subject("Happy new async year")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Be happy with async!"))
        .unwrap();

    match mail_transport.send_email(email).await {
        Ok(_) => log::info!("email sent"),
        Err(e) => log::error!("{}", e),
    }
}