
use mqtt::ffimqttasync;

use std::mem;
use libc::{c_char, c_void};
use std::ffi::CString;
use std::ptr;


pub enum PersistenceType {
    Default = 0,
    Nothing = 1,
    User = 2,
}

pub enum ConnectReturnCode {
    Success = 0,
    UnacceptableProtocol = 1,
    IdentifierRejected = 2,
    ServerUnavailable = 3,
    BadUsernameOrPassword = 4,
    NotAuthorized = 5,
    Reserved = 6,
}

pub struct AsyncClient {
    client: ffimqttasync::MQTTAsync,
}

impl AsyncClient {

    pub fn new(address: &str, clientid: &str, persistence: PersistenceType) -> AsyncClient {
        let mut fficlient: ffimqttasync::MQTTAsync = unsafe{mem::zeroed()};
        let mut persistence_context: c_void = unsafe{mem::zeroed()};

        let c_url = CString::new(address).unwrap();
        let c_clientid = CString::new(clientid).unwrap();

        let array_url = c_url.as_bytes_with_nul();
        let array_clientid = c_clientid.as_bytes_with_nul();

        unsafe {
            ffimqttasync::MQTTAsync_create(&mut fficlient,
                                           mem::transmute::<&u8, *const c_char>(&array_url[0]),
                                           mem::transmute::<&u8, *const c_char>(&array_clientid[0]),
                                           persistence as i32,
                                           &mut persistence_context);
        }

        let client = AsyncClient {
            client: fficlient,
        };

        client
    }

    pub fn connect(&mut self, options: &mut AsyncConnectOptions) -> ConnectReturnCode {
        unsafe {
            ffimqttasync::MQTTAsync_setCallbacks(self.client,
                                                 ptr::null_mut(),
                                                 Some(Self::disconnected),
                                                 Some(Self::received),
                                                 None,
                                               );
        }

        // fill in FFI private struct
        options.options.keepAliveInterval = options.keep_alive_interval;
        options.options.cleansession = options.cleansession;
        options.options.maxInflight = options.max_in_flight;
        options.options.connectTimeout = options.connect_timeout;
        options.options.retryInterval = options.retry_interval;

        // register callbacks
        options.options.onSuccess = Some(Self::connect_succeeded);
        options.options.onFailure = Some(Self::connect_failed);

        let mut error = 0;
        unsafe {
            error = ffimqttasync::MQTTAsync_connect(self.client, &options.options);
        }

        match error {
            0 => ConnectReturnCode::Success,
            1 => ConnectReturnCode::UnacceptableProtocol,
            2 => ConnectReturnCode::IdentifierRejected,
            3 => ConnectReturnCode::ServerUnavailable,
            4 => ConnectReturnCode::BadUsernameOrPassword,
            5 => ConnectReturnCode::NotAuthorized,
            _ => ConnectReturnCode::Reserved,
        }
    }

    extern "C" fn connect_succeeded(context: *mut ::libc::c_void, response: *mut ffimqttasync::MQTTAsync_successData) -> () {
        println!("connect succeeded");
    }

    extern "C" fn connect_failed(context: *mut ::libc::c_void, response: *mut ffimqttasync::MQTTAsync_failureData) -> () {
        println!("connect failed");
    }

    extern "C" fn disconnected(context: *mut c_void, cause: *mut c_char) -> () {
        println!("disconnected");

    }

    extern "C" fn received(context: *mut ::libc::c_void, topic_name: *mut ::libc::c_char, topic_len: ::libc::c_int, message: *mut ffimqttasync::MQTTAsync_message) -> i32 {
        println!("received");
        
        42
    }
}

pub struct AsyncConnectOptions {
    options: ffimqttasync::MQTTAsync_connectOptions,

    pub keep_alive_interval: i32,
    pub cleansession: i32,
    pub max_in_flight: i32,
    pub connect_timeout: i32,
    pub retry_interval: i32,
}

impl AsyncConnectOptions {

    pub fn new() -> AsyncConnectOptions {
        let ffioptions = ffimqttasync::MQTTAsync_connectOptions {
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
            onSuccess: None,
            onFailure: None,
            context: ptr::null_mut(),
            serverURIcount: 0,
            serverURIs: ptr::null_mut(),
            MQTTVersion: 0,
        };

        let options = AsyncConnectOptions {
            options: ffioptions,

            keep_alive_interval: 20,
            cleansession: 1,
            max_in_flight: 10,
            connect_timeout: 30,
            retry_interval: 0,
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
