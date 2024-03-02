use chrono::{DateTime, Utc};
use lettre::{message::header::ContentType, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use super::api::{BlueRideUser, EmailPayload, ErrorTypes};


#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AuthNotification {
    token: String,
    eov: DateTime<Utc>,
}

impl EmailPayload for AuthNotification {

    #[tracing::instrument(skip_all)]
    fn build_email(&self, target: &BlueRideUser) -> Result<Message, ()> {
        let to = format!("{} <{}>", target.name, target.email);
        let from = "BlueRide <blueride@hackduke.org>".to_owned();

        let content = format!(
            "Dear {},
        
        Your authentication code is {}. It is valid until {}.
        
        Best,
        BlueRide",
            target.name,
            self.token,
            self.eov
        );

        let email = Message::builder();
        Ok(email
            .from(from.parse().unwrap())
            .to(to.parse().unwrap())
            .subject("BlueRide Login Token")
            .header(ContentType::TEXT_PLAIN)
            .body(content)
            .unwrap())
    }

    #[tracing::instrument]
    async fn dispatch_email(&self, target: &BlueRideUser, mailer: &AsyncSmtpTransport<Tokio1Executor>)  -> Result<(), ErrorTypes>{
        if let Ok(message) = self.build_email(target) {
        if mailer.send(message).await.is_err() {
            log::error!("Failed to send email");
            return Err(ErrorTypes::ServiceDown);
        }
    } else {
        log::error!("Failed to build message");
        return Err(ErrorTypes::ParseFailure);
    }
    log::info!("Successfully auth token sent email to {}", &target.email);
    Ok(())
    }
}