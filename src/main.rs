mod tasks;
mod mailer;
use std::{env, error::Error, time::Duration};

use lapin::{
    message::DeliveryResult, options::{BasicConsumeOptions, BasicQosOptions, QueueDeclareOptions}, types::FieldTable, Connection, ConnectionProperties
};
use log;
use std::sync::Arc;
use std::process::exit;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::prelude::*;
use crate::tasks::api::handle_queue_request;
use rustls::crypto::aws_lc_rs;

async fn rabbit_mq(uri: &str) -> Result<(), Box<dyn Error>> {
    let options = ConnectionProperties::default()
        // Use tokio executor and reactor.
        // At the moment the reactor is only available for unix.
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);

    let connection = Connection::connect(uri, options).await?;
    let channel = connection.create_channel().await?;

    connection.on_error(|err| {
        log::error!("RMQ Connection Error: {}", err);
        exit(1);
    });

    channel.basic_qos(10, BasicQosOptions::default()).await?;
    let mut options = QueueDeclareOptions::default();
    options.durable = true;
    let _queue = channel
        .queue_declare(
            "notification_queue",
            options,
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
    aws_lc_rs::default_provider().install_default().expect("Failed to install default crypto provider");

    let mut filter = LevelFilter::DEBUG;
    if cfg!(debug_assertions) {
        filter = LevelFilter::DEBUG;
    }

    
    let _guard = sentry::init((
        env::var("SENTRY_DSN").expect("SENTRY_DSN not configured"),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 1.0,
            ..Default::default()
        },
    ));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .with(filter)
        .init();

    let uri = env::var("RABBITMQ_URI").unwrap_or("amqp://localhost:5672".to_string());
    while let Err(err) = rabbit_mq(&uri).await {
        tokio::time::sleep(Duration::from_secs(10)).await;
        log::error!("RabbitMQ Error: {}", err);
        let _ = rabbit_mq(&uri).await;
    }
}