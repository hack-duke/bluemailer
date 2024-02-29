mod api;
mod mailer;
use std::{env, error::Error};

use lapin::{
    message::DeliveryResult,
    options::{BasicConsumeOptions, BasicQosOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use log;

// fn retry_rabbitmq() {
//     std::thread::sleep(std::time::Duration::from_millis(2000));
//     log::info!("Reconnecting to rabbitmq");
//     try_rabbitmq();
// }

// async fn try_rabbitmq() {
//     // tokio::spawn(async move {
//     if let Err(err) = rabbit_mq().await {
//         log::error!("Error: {}", err);
//         retry_rabbitmq();
//     }
//     // });
// }

async fn rabbit_mq() -> Result<(), Box<dyn Error>> {
    let uri = "amqp://localhost:5672";
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

    consumer.set_delegate(move |delivery: DeliveryResult| async move {
        let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME not in env");
        let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not in env");
        let smtp_host = env::var("SMTP_HOST").expect("SMTP_HOST not in env");
        log::info!("Loaded SMTP configuration");
        let smtp_mailer = mailer::Mailer::create_mailer(smtp_username, smtp_password, smtp_host);
        api::handle_queue_request(delivery, smtp_mailer.mailer).await;
    });

    // channel
    //     .basic_publish(
    //         "",
    //         "notifications.blueride",
    //         BasicPublishOptions::default(),
    //         b"Hello world!",
    //         BasicProperties::default(),
    //     )
    //     .await
    //     .unwrap()
    //     .await
    //     .unwrap();

    log::info!("Awaiting next steps");

    std::future::pending::<()>().await;
    Ok(())
}

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let _guard = sentry::init((***REMOVED***, sentry::ClientOptions {
        release: sentry::release_name!(),
        ..Default::default()
      }));
    let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME not in env");
    let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not in env");
    let smtp_host = env::var("SMTP_HOST").expect("SMTP_HOST not in env");
    log::info!("Loaded SMTP configuration");
    let _smtp_mailer = mailer::Mailer::create_mailer(smtp_username, smtp_password, smtp_host);
    // mailer::send_test_email(&smtp_mailer).await;
    if let Err(err) = rabbit_mq().await {
        log::error!("Error: {}", err);
        let _ = rabbit_mq().await;
    }
    // rabbit_mq().await;
}
