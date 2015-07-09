use mqtt::async::PersistenceType;
use mqtt::async::Qos;

#[macro_use]
extern crate log;
extern crate fern;
extern crate time;
extern crate mqtt;

use std::thread;
use std::char;

static MQTT_SERVER_ADDRESS: &'static str = "tcp://localhost:1883";
static MQTT_TOPIC: &'static str = "TestTopic";

fn main() {
    // setup fern logger
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

    // start processing
    info!("sendreceive test started");

    let mut connect_options = mqtt::async::AsyncConnectOptions::new();
    match mqtt::async::AsyncClient::new(&MQTT_SERVER_ADDRESS, "TestClientId", PersistenceType::Nothing) {
        Ok(mut client) => {
            match client.connect(&mut connect_options) {
                Ok(_) => {
                    let mut data = Vec::new();
                    match client.subscribe(&MQTT_TOPIC, Qos::FireAndForget) {
                        Ok(_) => {
                            for i in 0..10 {
                                info!("data len: {}", i);
                                data.push(char::from_digit(i % 10, 10).unwrap() as u8);
                                client.send(&data, &MQTT_TOPIC, Qos::FireAndForget).unwrap();
                                for message in client.messages() {
                                    info!("{:?}", message);
                                }

                                thread::sleep_ms(200);
                            }
                        },
                        Err(e) => {error!("Error subscribing: {:?}", e)}
                    }
                },
                Err(e) => {error!("Error {:?} returned when trying to connect to {:}. Make sure MQTT broker called 'mosquitto' is running!", e, &MQTT_SERVER_ADDRESS)}
            }
        },
        Err(e) => {error!("Error creating: {:?}", e)}
    }

    info!("sendreceive test ended");
}
