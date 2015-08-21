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
    info!("loopback test started");

    let mut data = Vec::new();
    let topic = "TestTopic";
    match setup_mqtt("tcp://localhost:1883", &topic, "TestClientId") {
        Ok(mut client) => {
            for i in 0..10 {
                info!("data len: {}", i);
                data.push(char::from_digit(i % 10, 10).unwrap() as u8);
                client.send(&data, &topic, Qos::FireAndForget).unwrap();
                for message in client.messages() {
                    info!("{:?}", message);
                }
                thread::sleep_ms(200);
            }

            let disconnect_options = AsyncDisconnectOptions::new();
            client.disconnect(&disconnect_options).unwrap();
            },
        Err(e) => error!("{}; raw error: {}", e.description(), e)
    }
    info!("loopback test ended");
}
