use std::{fmt::Debug, string::String};

use lapin::{
    message::DeliveryResult,
    options::{BasicAckOptions, BasicNackOptions},
};
use lettre::{
    message::header::ContentType, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct BlueRideUser {
    name: String,
    email: String,
    phone_number: String,
    apn_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
enum NotificationChannel {
    Email,
    APN,
}

#[derive(Serialize, Deserialize, Debug)]
struct GroupNotification {
    match_id: String,
    group: Vec<BlueRideUser>,
    datetime_start: String,
    datetime_end: String,
}

enum ErrorTypes {
    ParseFailure,
    ServiceDown,
    UnknownError,
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
}

#[derive(Serialize, Deserialize, Debug)]
struct BlueRideNotification {
    target_user: BlueRideUser,
    channels: Vec<NotificationChannel>,
    payload: NotificationPurpose,
}

pub async fn handle_queue_request(
    delivery: DeliveryResult,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
) {
    let delivery = match delivery {
        // Carries the delivery alongside its channel
        Ok(Some(delivery)) => delivery,
        // The consumer got canceled
        Ok(None) => return,
        // Carries the error and is always followed by Ok(None)
        Err(error) => {
            log::error!("Failed to consume queue message {}", error);
            return;
        }
    };

    // Do something with the delivery data (The message payload)
    log::info!(
        "Received message",
    );

    if let Ok(p) = serde_json::from_slice::<BlueRideNotification>(&delivery.data) {
        let e = match p.payload {
            NotificationPurpose::Matched { data } => {
                dispatch_match(data, p.target_user, &mailer).await
            }
            NotificationPurpose::Canceled { data, reason } => dispatch_cancel(data, reason).await,
        };
    } else {
        delivery
            .nack(BasicNackOptions {
                multiple: true,
                requeue: false,
            })
            .await
            .expect("Failed to send no ack");
    }

    delivery
        .ack(BasicAckOptions { multiple: true })
        .await
        .expect("Failed to ack send_webhook_event message");
}

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
        return Err(ErrorTypes::ParseFailure);
    }
    log::info!("Successfully sent email to {}", &target.email);
    Ok(())
}

fn build_match_email(data: GroupNotification, target: &BlueRideUser) -> Result<Message, ()> {
    let to = format!("{} <{}>", target.name, target.email);
    let from = "BlueRide <blueride@hackduke.org>".to_owned();

    let content = format!(
        "Dear {},
    You have been matched for a ride on {} with the following individuals:
        {}",
        target.name,
        data.datetime_start,
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

fn build_list_of_individuals(group: &Vec<BlueRideUser>) -> String {
    let mut result = "".to_owned();
    for user in group {
        let s = format!("- {}: {}\n", user.name, user.phone_number);
        result += &s;
    }
    result
}

async fn dispatch_cancel(data: GroupNotification, reason: String) -> Result<(), ErrorTypes> {
    Ok(())
}
