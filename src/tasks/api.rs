use std::{fmt::Debug, string::String, sync::Arc};

use chrono::{DateTime, Utc};
use lapin::{
    message::{Delivery, DeliveryResult},
    options::{BasicAckOptions, BasicNackOptions, BasicRejectOptions},
};
use lettre::{
    message::header::ContentType, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use sentry::Transaction;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;
use tracing::{event, Level};

use super::auth_token::AuthNotification;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BlueRideUser {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) phone_number: String,
    pub(crate) apn_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum NotificationChannel {
    Email,
    APN,
}

#[derive(Serialize, Deserialize, Debug)]
struct GroupNotification {
    match_id: String,
    group: Vec<BlueRideUser>,
    datetime_start: DateTime<Utc>,
    datetime_end: DateTime<Utc>,
}

#[derive(Debug)]
pub(crate) enum ErrorTypes {
    ParseFailure,
    ServiceDown,
    EmailParseFailure,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum NotificationPurpose {
    Matched {
        data: GroupNotification,
    },

    Canceled {
        data: GroupNotification,
        reason: String,
    },

    AuthToken {
        data: AuthNotification
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct BlueRideNotification {
    target_user: BlueRideUser,
    channels: Vec<NotificationChannel>,
    payload: NotificationPurpose,
    trace_id: Option<String>
}

pub(crate) trait EmailPayload {
    fn build_email(&self, target: &BlueRideUser) -> Result<Message, ()>;
    async fn dispatch_email(&self, target: &BlueRideUser, mailer: &AsyncSmtpTransport<Tokio1Executor>)  -> Result<(), ErrorTypes>;
}

#[tracing::instrument(skip_all)]
async fn delay(delivery: &Delivery) {
    tokio::time::sleep(Duration::from_secs(10)).await;
    delivery
        .nack(BasicNackOptions {
            requeue: true,
            multiple: false,
        })
        .await
        .expect("Failed to send reject");
}

#[tracing::instrument(skip_all)]
pub async fn handle_queue_request(
    delivery: DeliveryResult,
    mailer: Arc<AsyncSmtpTransport<Tokio1Executor>>,
    transaction: &Transaction,
) {
    let delivery = match delivery {
        // Carries the delivery alongside its channel
        Ok(Some(delivery)) => delivery,
        // The consumer got canceled
        Ok(None) => return,
        // Carries the error and is always followed by Ok(None)
        Err(error) => {
            log::error!("Failed to consume queue message {}", error);
            transaction.set_status(sentry::protocol::SpanStatus::Aborted);
            return;
        }
    };

    // Do something with the delivery data (The message payload)
    log::info!("Received message",);

    if let Ok(p) = serde_json::from_slice::<BlueRideNotification>(&delivery.data) {
        sentry::configure_scope(|scope| {
            scope.set_transaction(p.trace_id.as_deref());
        });

        let e = match p.payload {
            NotificationPurpose::Matched { data } => {
                dispatch_match(data, p.target_user, &mailer).await
            }
            NotificationPurpose::Canceled { data, reason } => {
                dispatch_cancel(data, reason, &p.target_user, &mailer).await
            }
            NotificationPurpose::AuthToken { data } => {
                data.dispatch_email(&p.target_user, &mailer).await
            }
        };
        if let Err(err) = e {
            log::error!("Encountered internal error: {:?}", err);
            match err {
                ErrorTypes::ParseFailure | ErrorTypes::EmailParseFailure => {
                    transaction.set_status(sentry::protocol::SpanStatus::InvalidArgument);
                    delivery
                        .reject(BasicRejectOptions { requeue: false })
                        .await
                        .expect("Failed to send reject");
                }
                ErrorTypes::ServiceDown => {
                    transaction.set_status(sentry::protocol::SpanStatus::InternalError);
                    let s = transaction.start_child("waiting", "10s delay before requeue");
                    delay(&delivery).await;
                    s.finish();
                }
            }
            return;
        }
    } else {
        transaction.set_status(sentry::protocol::SpanStatus::InvalidArgument);
        log::warn!(
            "Failed to decode JSON: {:?}",
            String::from_utf8(delivery.data.clone())
        );
        event!(Level::WARN, "Failed to decode JSON");
        delivery
            .reject(BasicRejectOptions { requeue: false })
            .await
            .expect("Failed to send reject");
        return;
    }
    transaction.set_status(sentry::protocol::SpanStatus::Ok);
    delivery
        .ack(BasicAckOptions { multiple: false })
        .await
        .expect("Failed to ack send_webhook_event message");
}

#[tracing::instrument]
async fn dispatch_match(
    data: GroupNotification,
    target: BlueRideUser,
    mailer: &AsyncSmtpTransport<Tokio1Executor>,
) -> Result<(), ErrorTypes> {
    if let Ok(message) = build_match_email(data, &target) {
        if mailer.send(message).await.is_err() {
            log::error!("Failed to send email");
            return Err(ErrorTypes::ServiceDown);
        }
    } else {
        log::error!("Failed to build message");
        return Err(ErrorTypes::EmailParseFailure);
    }
    log::info!("Successfully sent email to {}", &target.email);
    Ok(())
}

#[tracing::instrument]
fn build_match_email(data: GroupNotification, target: &BlueRideUser) -> Result<Message, ()> {
    let to = format!("{} <{}>", target.name, target.email);
    let from = "BlueRide <blueride@hackduke.org>".to_owned();

    let et_start = data.datetime_start.with_timezone(&chrono_tz::US::Eastern);
    let et_end = data.datetime_end.with_timezone(&chrono_tz::US::Eastern);
    
    // Format the Eastern Time
    let ft_start = et_start.format("%d/%m/%Y %I:%M%P %Z").to_string();
    let ft_end = et_end.format("%d/%m/%Y %I:%M%P %Z").to_string();


    let content = format!(
        "Dear {},
    You have been matched for a ride in a time range from {} to {} with the following individuals:

        {}
    
    Please check the app for more details. You may have received an earlier email. This email is to indicate one/more persons have now joined the group.
    
    Best,
    BlueRide",
        target.name,
        ft_start,
        ft_end,
        build_list_of_individuals(&data.group)
    );

    let email = Message::builder();
    Ok(email
        .from(from.parse().unwrap())
        .to(to.parse().unwrap())
        .subject("BlueRide Match Found")
        .header(ContentType::TEXT_PLAIN)
        .body(content)
        .unwrap())
}

#[tracing::instrument]
fn build_cancel_email(
    data: GroupNotification,
    target: &BlueRideUser,
    reason: String,
) -> Result<Message, ()> {
    let to = format!("{} <{}>", target.name, target.email);
    let from = "BlueRide <blueride@hackduke.org>".to_owned();

    let et_start = data.datetime_start.with_timezone(&chrono_tz::US::Eastern);
    let et_end = data.datetime_end.with_timezone(&chrono_tz::US::Eastern);
    
    // Format the Eastern Time
    let ft_start = et_start.format("%d/%m/%Y %I:%M%P %Z").to_string();
    let ft_end = et_end.format("%d/%m/%Y %I:%M%P %Z").to_string();

    let content = format!(
        "Dear {},
    Your matched ride in the time range from {} to {} with the following individuals has changed due to a user leaving:

        {}

    Reason: {}
    
    Best,
    BlueRide",
        target.name,
        ft_start,
        ft_end,
        build_list_of_individuals(&data.group),
        reason
    );

    let email = Message::builder();
    Ok(email
        .from(from.parse().unwrap())
        .to(to.parse().unwrap())
        .subject("A user has left your BlueRide match.")
        .header(ContentType::TEXT_PLAIN)
        .body(content)
        .unwrap())
}

#[tracing::instrument]
fn build_list_of_individuals(group: &Vec<BlueRideUser>) -> String {
    let mut result = "".to_owned();
    for user in group {
        let s = format!("- {}: {}\n", user.name, user.phone_number);
        result += &s;
    }
    result
}

#[tracing::instrument]
async fn dispatch_cancel(
    data: GroupNotification,
    reason: String,
    target: &BlueRideUser,
    mailer: &AsyncSmtpTransport<Tokio1Executor>,
) -> Result<(), ErrorTypes> {
    if let Ok(message) = build_cancel_email(data, &target, reason) {
        if mailer.send(message).await.is_err() {
            log::error!("Failed to send email");
            return Err(ErrorTypes::ServiceDown);
        }
    } else {
        log::error!("Failed to build message");
        return Err(ErrorTypes::ParseFailure);
    }
    log::info!("Successfully sent email to {}", &target.email);
    Ok(())
}
