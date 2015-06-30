
use mqtt::ffiasync;
use eventual;

use std::mem;
use libc::{c_char, c_int, c_void};
use std::ffi::CString;
use std::ffi::CStr;
use std::ptr;
use std::slice;


pub enum PersistenceType {
    Default = 0,
    Nothing = 1,
    User = 2,
}

#[derive(Debug)]
pub enum ConnectReturnCode {
    Success = 0,
    UnacceptableProtocol = 1,
    IdentifierRejected = 2,
    ServerUnavailable = 3,
    BadUsernameOrPassword = 4,
    NotAuthorized = 5,
    Reserved = 6,
}

#[derive(Debug)]
pub enum MqttResult {
    Success,
    Error(i32),
}

#[derive(Debug)]
pub enum Qos {
    FireAndForget = 0,
    AtLeastOnce = 1,
    OnceAndOneOnly = 2,
}

pub struct AsyncClient {
    handle: ffiasync::MQTTAsync,
    context: *mut c_void,
    pub future: (eventual::Complete<i32, ()>, eventual::Future<i32, ()>),
    pub random: i32,
}

impl AsyncClient {

    /// Ensures the FFI struct is consistent for callback invocation.
    ///
    /// Because the user may move this struct, the `context` pointer
    /// passed back to FFI callbacks might be invalidated. This function
    /// should be called before FFI actions that might fire callbacks to ensure
    /// the self-pointer is valid.
    fn callback_setup(&mut self) {
        let context = self as *mut _ as *mut c_void;
        self.context = context;
    }

    pub fn new(address: &str, clientid: &str, persistence: PersistenceType) -> Result<AsyncClient, MqttResult> {
        let mut handle = unsafe{mem::zeroed()};
        let mut persistence_context: c_void = unsafe{mem::zeroed()};

        let c_url = CString::new(address).unwrap();
        let c_clientid = CString::new(clientid).unwrap();

        let array_url = c_url.as_bytes_with_nul();
        let array_clientid = c_clientid.as_bytes_with_nul();

        let (c, f) = eventual::Future::<i32, ()>::pair();

        let mut error = 0;
        unsafe {
            error = ffiasync::MQTTAsync_create(&mut handle,
                                           mem::transmute::<&u8, *const c_char>(&array_url[0]),
                                           mem::transmute::<&u8, *const c_char>(&array_clientid[0]),
                                           persistence as i32,
                                           &mut persistence_context);
        }

        match error {
            0 => {
                let client = AsyncClient {
                    handle: handle,
                    context: ptr::null_mut(),
                    future: (c, f),
                    random: 33,
                };

                Ok(client)
            },

            err => Err(MqttResult::Error(err))
        }
    }

    pub fn connect(&mut self, options: &mut AsyncConnectOptions) -> Result<ConnectReturnCode, ConnectReturnCode> {
        self.callback_setup();

        unsafe {
            ffiasync::MQTTAsync_setCallbacks(self.handle,
                                                 self.context,
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
        options.options.context = self.context;
        options.options.onSuccess = Some(Self::connect_succeeded);
        options.options.onFailure = Some(Self::connect_failed);

        let mut error = 0;
        unsafe {
            error = ffiasync::MQTTAsync_connect(self.handle, &options.options);
        }

        match error {
            0 => Ok(ConnectReturnCode::Success),
            1 => Err(ConnectReturnCode::UnacceptableProtocol),
            2 => Err(ConnectReturnCode::IdentifierRejected),
            3 => Err(ConnectReturnCode::ServerUnavailable),
            4 => Err(ConnectReturnCode::BadUsernameOrPassword),
            5 => Err(ConnectReturnCode::NotAuthorized),
            _ => Err(ConnectReturnCode::Reserved),
        }
    }

    extern "C" fn connect_succeeded(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_successData) -> () {
        println!("connect succeeded");
        assert!(!context.is_null());

        let selfclient: &mut AsyncClient = unsafe {mem::transmute(context)};

        //selfclient.future.0.complete(42);
        selfclient.random = 69;
    }

    extern "C" fn connect_failed(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_failureData) -> () {
        println!("connect failed");
        assert!(!context.is_null());
    }

    extern "C" fn disconnected(context: *mut c_void, cause: *mut c_char) -> () {
        println!("disconnected");
        assert!(!context.is_null());
    }

    pub fn is_connected(&mut self) -> bool {
        let mut ret = 0;
        unsafe {
            ret = ffiasync::MQTTAsync_isConnected(self.handle);
        }

        match ret {
            1 => true,
            _ => false,
        }
    }

    pub fn send(&mut self, data: &[u8], topic: &str, qos: Qos) -> Result<(), MqttResult>{
        self.callback_setup();

        let mut responseoption = ffiasync::MQTTAsync_responseOptions {
            struct_id: ['M' as i8, 'Q' as i8, 'T' as i8, 'R' as i8],
            struct_version: 0,
            onSuccess: Some(Self::send_succeeded),
            onFailure: None,
            context: self.context,
            token: 0,
        };

        let mut message = ffiasync::MQTTAsync_message {
            struct_id: ['M' as i8, 'Q' as i8, 'T' as i8, 'M' as i8],
            struct_version: 0,
            payloadlen: data.len() as i32,
            payload: unsafe {mem::transmute::<&u8, *mut c_void>(&data[0])},
            qos: qos as c_int,
            retained: 0,
            dup: 0,
            msgid: 0,
        };

        let c_topic = CString::new(topic).unwrap();
        let array_topic = c_topic.as_bytes_with_nul();

        let mut error = 0;
        unsafe {
            error = ffiasync::MQTTAsync_sendMessage(self.handle,
                                                        mem::transmute::<&u8, *const c_char>(&array_topic[0]),
                                                        &mut message,
                                                        &mut responseoption,
                                                        );
        }

        match error {
            0 => Ok(()),
            err => Err(MqttResult::Error(err)),
        }
    }

    extern "C" fn send_succeeded(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_successData) -> () {
        println!("send succeeded");
        assert!(!context.is_null());
    }

    pub fn subscribe(&mut self, topic: &str, qos: Qos) -> Result<(), MqttResult> {
        self.callback_setup();

        let mut responseoption = ffiasync::MQTTAsync_responseOptions {
            struct_id: ['M' as i8, 'Q' as i8, 'T' as i8, 'R' as i8],
            struct_version: 0,
            onSuccess: Some(Self::subscribe_succeeded),
            onFailure: Some(Self::subscribe_failed),
            context: self.context,
            token: 0,
        };

        let c_topic = CString::new(topic).unwrap();
        let array_topic = c_topic.as_bytes_with_nul();

        let c_qos: i32 = match qos {
            Qos::FireAndForget => 0,
            Qos::AtLeastOnce => 1,
            Qos::OnceAndOneOnly => 2,
        };

        let mut error = 0;
        unsafe {
            error = ffiasync::MQTTAsync_subscribe(self.handle,
                                                      mem::transmute::<&u8, *const c_char>(&array_topic[0]),
                                                      c_qos,
                                                      &mut responseoption,
                                                      );
        }

        match error {
            0 => Ok(()),
            err => Err(MqttResult::Error(err)),
        }
    }

    extern "C" fn subscribe_succeeded(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_successData) -> () {
        println!("subscribe succeeded");
        assert!(!context.is_null());
    }

    extern "C" fn subscribe_failed(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_failureData) -> () {
        println!("subscribe failed");
        assert!(!context.is_null());
    }

    extern "C" fn received(context: *mut ::libc::c_void, topic_name: *mut ::libc::c_char, topic_len: ::libc::c_int, amessage: *mut ffiasync::MQTTAsync_message) -> i32 {
        let c_topic = unsafe {CStr::from_ptr(topic_name).to_bytes()};
        let topic = String::from_utf8(c_topic.to_vec()).unwrap();
        assert_eq!(topic.len(), topic_len as usize);

        assert!(!amessage.is_null());
        let transmessage: &mut ffiasync::MQTTAsync_message = unsafe {mem::transmute(amessage)};
        let message: &[u8] = unsafe {
                slice::from_raw_parts(transmessage.payload as *mut u8, transmessage.payloadlen as usize)
        };

        let strmessage = String::from_utf8(message.to_vec()).unwrap();
        println!("received from topic: {}", topic);
        println!("       utf8 message: {}", strmessage);
        println!("        raw message: {:?}", message);

        let mut msg = amessage;
        unsafe{ffiasync::MQTTAsync_freeMessage(&mut msg)};
        unsafe{ffiasync::MQTTAsync_free(mem::transmute(topic_name))};

        1
    }
}

impl Drop for AsyncClient {
    fn drop(&mut self) {
        unsafe{ffiasync::MQTTAsync_destroy(&mut self.handle)};
    }
}

pub struct AsyncConnectOptions {
    options: ffiasync::MQTTAsync_connectOptions,

    pub keep_alive_interval: i32,
    pub cleansession: i32,
    pub max_in_flight: i32,
    pub connect_timeout: i32,
    pub retry_interval: i32,
}

impl AsyncConnectOptions {

    pub fn new() -> AsyncConnectOptions {
        let ffioptions = ffiasync::MQTTAsync_connectOptions {
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
    options: ffiasync::MQTTAsync_disconnectOptions,
}

impl AsyncDisconnectOptions {

    pub fn new() -> ffiasync::MQTTAsync_disconnectOptions {
        let options = ffiasync::MQTTAsync_disconnectOptions {
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
    options: ffiasync::MQTTAsync_message,
}

impl AsyncMessage {

    pub fn new() -> ffiasync::MQTTAsync_message {
        let message = ffiasync::MQTTAsync_message {
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
