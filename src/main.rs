use anyhow::{anyhow, Result};
use futures_lite::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, BasicQosOptions},
    types::{AMQPValue, FieldTable},
    BasicProperties, Connection, ConnectionProperties,
};
use tracing::{info, trace};

use crate::sdapi::SDApi;

mod sdapi;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt::init();

    let addr = if let Ok(addr) = std::env::var("LAILAI_MQ_ADDR") {
        addr
    } else {
        return Err(anyhow!("没有配置LAILAI_MQ_ADDR"));
    };

    let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;

    info!("MQ已连接");

    let channel = conn.create_channel().await?;

    channel.basic_qos(1, BasicQosOptions::default()).await?;
    trace!("qos设置为1");

    let mut consumer = channel
        .basic_consume(
            "sdqueue",
            "lailaisd",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let sdapi = SDApi::init();

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        let msg = String::from_utf8(delivery.data)?;

        info!("正在生成，prompts:{}", msg);

        let msg: Message = serde_json::from_str(&msg)?;

        if let Ok(data) = sdapi.clone().txt2img(&msg.tag).await {
            channel
                .basic_publish(
                    "",
                    &format!("sd.callback.{}", msg.uin),
                    BasicPublishOptions::default(),
                    &data,
                    get_properties(msg),
                )
                .await?;
        }
        channel
            .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
            .await?;
    }

    Ok(())
}

fn get_properties(msg: Message) -> BasicProperties {
    let prop = BasicProperties::default();

    let mut table = FieldTable::default();

    table.insert("send_to".into(), AMQPValue::LongLongInt(msg.send_to));
    table.insert("from_uin".into(), AMQPValue::LongLongInt(msg.from_uin));
    table.insert(
        "send_type".into(),
        AMQPValue::LongString(msg.send_type.into()),
    );

    prop.with_headers(table)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Message {
    from_uin: i64,
    send_to: i64,
    send_type: String,
    tag: String,
    uin: i64,
}
