
use mqtt::ffimqttasync;

use std::mem;
use libc::{c_char, c_void};
use std::ffi::CString;
use std::ptr;

#[allow(non_camel_case_types)]
pub enum PersistentType {
    PERSISTENCE_DEFAULT = 0,
    PERSISTENCE_NONE = 1,
    PERSISTENCE_USER = 2,
}

pub struct AsyncClient {
    client: ffimqttasync::MQTTAsync,
}

impl AsyncClient {

    pub fn new(url: &str, clientid: &str, persistence_type: PersistentType) -> ffimqttasync::MQTTAsync {
        let mut client: ffimqttasync::MQTTAsync = unsafe{mem::zeroed()};
        let mut persistence_context: c_void = unsafe{mem::zeroed()};

        let c_url = CString::new(url).unwrap();
        let c_clientid = CString::new(clientid).unwrap();

        let array_url = c_url.as_bytes_with_nul();
        let array_clientid = c_clientid.as_bytes_with_nul();

        unsafe {
            ffimqttasync::MQTTAsync_create(&mut client,
                                               mem::transmute::<&u8, *const c_char>(&array_url[0]),
                                               mem::transmute::<&u8, *const c_char>(&array_clientid[0]),
                                               persistence_type as i32,
                                               &mut persistence_context);
                }

        client
    }
}

pub struct AsyncConnectOptions {
    options: ffimqttasync::MQTTAsync_connectOptions,
}

impl AsyncConnectOptions {

    pub fn new() -> ffimqttasync::MQTTAsync_connectOptions {
        let options = ffimqttasync::MQTTAsync_connectOptions {
            struct_id: ['M' as i8, 'Q' as i8, 'T' as i8, 'C' as i8],
            struct_version: 3,
            keepAliveInterval: 60,
            cleansession: 1,
            maxInflight: 10,
            will: ptr::null_mut(),
            username: ptr::null_mut(),
            password: ptr::null_mut(),
            connectTimeout: 30,
            retryInterval: 0,
            ssl: ptr::null_mut(),
            onSuccess: ptr::null_mut(),
            onFailure: ptr::null_mut(),
            context: ptr::null_mut(),
            serverURIcount: 0,
            serverURIs: ptr::null_mut(),
            MQTTVersion: 0,
        };

        options
    }
}

pub struct AsyncDisconnectOptions {
    options: ffimqttasync::MQTTAsync_disconnectOptions,
}

impl AsyncDisconnectOptions {

    pub fn new() -> ffimqttasync::MQTTAsync_disconnectOptions {
        let options = ffimqttasync::MQTTAsync_disconnectOptions {
            struct_id: ['M' as i8, 'Q' as i8, 'T' as i8, 'D' as i8],
            struct_version: 0,
            timeout: 0,
            onSuccess: ptr::null_mut(),
            onFailure: ptr::null_mut(),
            context: ptr::null_mut(),
        };

        options
    }
}

pub struct AsyncMessage {
    options: ffimqttasync::MQTTAsync_message,
}

impl AsyncMessage {

    pub fn new() -> ffimqttasync::MQTTAsync_message {
        let message = ffimqttasync::MQTTAsync_message {
            struct_id: ['M' as i8, 'Q' as i8, 'T' as i8, 'M' as i8],
            struct_version: 0,
            payloadlen: 0,
            payload: ptr::null_mut(),
            qos: 0,
            retained: 0,
            dup: 0,
            msgid: 0,
        };

        message
    }
}
