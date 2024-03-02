mod tasks;
mod mailer;
use std::{env, error::Error};

use lapin::{
    message::DeliveryResult,
    options::{BasicConsumeOptions, BasicQosOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use log;
use std::sync::Arc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::prelude::*;
use crate::tasks::api::handle_queue_request;

async fn rabbit_mq(uri: &str) -> Result<(), Box<dyn Error>> {
    let options = ConnectionProperties::default()
        // Use tokio executor and reactor.
        // At the moment the reactor is only available for unix.
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);

    let connection = Connection::connect(uri, options).await?;
    let channel = connection.create_channel().await?;

    channel.basic_qos(10, BasicQosOptions::default()).await?;

    let _queue = channel
        .queue_declare(
            "notification_queue",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let consumer = channel
        .basic_consume(
            "notification_queue",
            "notifications.#",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME not in env");
    let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not in env");
    let smtp_host = env::var("SMTP_HOST").expect("SMTP_HOST not in env");
    log::info!("Loaded SMTP configuration");
    let smtp_mailer = Arc::from(mailer::create_mailer(
        smtp_username.clone(),
        smtp_password.clone(),
        smtp_host.clone(),
    ));

    consumer.set_delegate(move |delivery: DeliveryResult| {
        let s = Arc::clone(&smtp_mailer);
        async move {
            let tx_ctx =
                sentry::TransactionContext::new("handle_notification_queue", "process request");
            let transaction = sentry::start_transaction(tx_ctx);
            // let mailer = &smtp_mailer.mailer;
            handle_queue_request(delivery, Arc::clone(&s), &transaction).await;
            transaction.finish();
        }
    });

    log::info!("Awaiting next steps");

    std::future::pending::<()>().await;
    Ok(())
}

#[tokio::main]
async fn main() {
    // simple_logger::SimpleLogger::new().env().init().unwrap();
    let filter = LevelFilter::DEBUG;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .with(filter)
        .init();
    let _guard = sentry::init((
        env::var("SENTRY_DSN").expect("SENTRY_DSN not configured"),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 1.0,
            ..Default::default()
        },
    ));

    let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME not in env");
    let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not in env");
    let smtp_host = env::var("SMTP_HOST").expect("SMTP_HOST not in env");
    log::info!("Loaded SMTP configuration");
    let _smtp_mailer = mailer::Mailer::create_mailer(smtp_username, smtp_password, smtp_host);
    // mailer::send_test_email(&smtp_mailer).await;
    let uri = env::var("RABBITMQ_URI").unwrap_or("amqp://localhost:5672".to_string());
    if let Err(err) = rabbit_mq(&uri).await {
        log::error!("Error: {}", err);
        let _ = rabbit_mq(&uri).await;
    }
    // rabbit_mq().await;
}