use chrono::{DateTime, Utc};
use lettre::{message::header::ContentType, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use super::api::{BlueRideUser, ErrorTypes};


#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AuthNotification {
    token: String,
    eov: DateTime<Utc>,
}

#[tracing::instrument(skip_all)]
fn build_auth_email(
    auth_package: AuthNotification,
    target: &BlueRideUser,
) -> Result<Message, ()> {
    let to = format!("{} <{}>", target.name, target.email);
    let from = "BlueRide <blueride@hackduke.org>".to_owned();

    let content = format!(
        "Dear {},
    
    Your authentication code is {}. It is valid until {}.
    
    Best,
    BlueRide",
        target.name,
        auth_package.token,
        auth_package.eov
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
pub async fn dispatch_token(
    auth_package: AuthNotification,
    target: &BlueRideUser,
    mailer: &AsyncSmtpTransport<Tokio1Executor>,
) -> Result<(), ErrorTypes> {
    if let Ok(message) = build_auth_email(auth_package, target) {
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