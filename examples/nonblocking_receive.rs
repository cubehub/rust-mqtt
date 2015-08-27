#[macro_use]
extern crate log;
extern crate fern;
extern crate time;
extern crate mqtt;

use std::thread;
use std::char;
use mqtt::async::{PersistenceType, Qos, MqttError, AsyncClient, AsyncConnectOptions, AsyncDisconnectOptions};
use std::error::Error;


fn conf_logger() {
    let logger_config = fern::DispatchConfig {
        format: Box::new(|msg: &str, level: &log::LogLevel, _location: &log::LogLocation| {
            let t = time::now();
            let ms = t.tm_nsec/1000_000;
            format!("{}.{:3} [{}] {}", t.strftime("%Y-%m-%dT%H:%M:%S").unwrap(), ms, level, msg)
        }),
        output: vec![fern::OutputConfig::stderr()],
        level: log::LogLevelFilter::Trace,
    };

    if let Err(e) = fern::init_global_logger(logger_config, log::LogLevelFilter::Trace) {
        panic!("Failed to initialize global logger: {}", e);
    }
}

fn setup_mqtt(server_address: &str, topic: &str, client_id: &str) -> Result<AsyncClient, MqttError> {
    let connect_options = AsyncConnectOptions::new();
    let mut client = try!(AsyncClient::new(server_address, client_id, PersistenceType::Nothing));
    try!(client.connect(&connect_options));
    try!(client.subscribe(topic, Qos::FireAndForget));
    Ok(client)
}

fn main() {
    // setup fern logger
    conf_logger();

    // start processing
    info!("non-blocking receive test started");
    info!("run: mosquitto_pub -t TestTopic -m somedata to send some messages to the test");

    let topic = "TestTopic";
    match setup_mqtt("tcp://localhost:1883", &topic, "TestClientId") {
        Ok(mut client) => {

            loop {
                info!("wait for a message..");
                let timeout_ms = Some(500);
                for message in client.messages(timeout_ms) {
                    info!("{:?}", message);
                }
            }

            let disconnect_options = AsyncDisconnectOptions::new();
            client.disconnect(&disconnect_options).unwrap();
            },
        Err(e) => error!("{}; raw error: {}", e.description(), e)
    }
    info!("non-blocking receive test ended");
}
