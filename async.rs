use mqtt::ffiasync;

use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;
use std::slice;
use std::sync::Barrier;


pub enum PersistenceType {
    Default = 0,
    Nothing = 1,
    User    = 2,
}

#[derive(Debug)]
pub enum ConnectReturnCode {
    Success               = 0,
    UnacceptableProtocol  = 1,
    IdentifierRejected    = 2,
    ServerUnavailable     = 3,
    BadUsernameOrPassword = 4,
    NotAuthorized         = 5,
    Reserved              = 6,
}

#[derive(Debug)]
pub enum ConnectErr {
    ReturnCode(ConnectReturnCode),
    Callback
}

#[derive(Debug, Copy, Clone)]
pub enum MqttError {
    Code(i32),
    Bad
}

#[derive(Debug)]
pub enum Qos {
    FireAndForget  = 0,
    AtLeastOnce    = 1,
    OnceAndOneOnly = 2,
}

#[derive(Debug, Copy, Clone)]
pub enum ActionResult {
    None,
    Ok,
    Err(MqttError)
}

pub struct AsyncClient {
    handle        : ffiasync::MQTTAsync,
    pub random    : i32,
    barrier       : Barrier,
    action_result : ActionResult
}
impl AsyncClient {

    /// Ensures the FFI struct is consistent for callback invocation.
    ///
    /// Because the user may move this struct, the `context` pointer
    /// passed back to FFI callbacks might be invalidated. This function
    /// should be called before FFI actions that might fire callbacks to ensure
    /// the self-pointer is valid.
    fn context(&mut self) -> *mut c_void {
        self as *mut _ as *mut c_void
    }

    pub fn new(address: &str, clientid: &str, persistence: PersistenceType) -> Result<AsyncClient, MqttError> {
        let mut handle                      = unsafe{mem::zeroed()};
        let mut persistence_context: c_void = unsafe{mem::zeroed()};

        let c_url          = CString::new(address).unwrap();
        let c_clientid     = CString::new(clientid).unwrap();
        let array_url      = c_url.as_bytes_with_nul();
        let array_clientid = c_clientid.as_bytes_with_nul();

        let mut error = unsafe {
            ffiasync::MQTTAsync_create(&mut handle,
                                       mem::transmute::<&u8, *const c_char>(&array_url[0]),
                                       mem::transmute::<&u8, *const c_char>(&array_clientid[0]),
                                       persistence as i32,
                                       &mut persistence_context)
        };

        match error {
            0   => { Ok( AsyncClient {
                        handle          : handle,
                        random          : 33,
                        barrier         : Barrier::new(2),
                        action_result   : ActionResult::None
                    })},
            err => Err(MqttError::Code(err))
        }
    }

    pub fn connect(&mut self, options: &mut AsyncConnectOptions) -> Result<(), ConnectErr> {
        unsafe {
            ffiasync::MQTTAsync_setCallbacks(self.handle,
                                             self.context(),
                                             Some(Self::disconnected),
                                             Some(Self::received),
                                             None);
        }

        // fill in FFI private struct
        options.options.keepAliveInterval = options.keep_alive_interval;
        options.options.cleansession      = options.cleansession;
        options.options.maxInflight       = options.max_in_flight;
        options.options.connectTimeout    = options.connect_timeout;
        options.options.retryInterval     = options.retry_interval;

        // register callbacks
        options.options.context   = self.context();
        options.options.onSuccess = Some(Self::connect_succeeded);
        options.options.onFailure = Some(Self::connect_failed);

        self.action_result = ActionResult::None;
        let mut error = unsafe {
            ffiasync::MQTTAsync_connect(self.handle, &options.options)
        };
        if error == 0 {
            self.barrier.wait();
            if self.is_connected() {
                Ok(())
            } else {
                Err(ConnectErr::Callback)
            }
        } else {
            Err(ConnectErr::ReturnCode(
                match error {
                    1 => ConnectReturnCode::UnacceptableProtocol,
                    2 => ConnectReturnCode::IdentifierRejected,
                    3 => ConnectReturnCode::ServerUnavailable,
                    4 => ConnectReturnCode::BadUsernameOrPassword,
                    5 => ConnectReturnCode::NotAuthorized,
                    _ => ConnectReturnCode::Reserved,
            }))
        }
    }

    extern "C" fn connect_succeeded(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_successData) -> () {
        println!("connect succeeded");
        assert!(!context.is_null());

        let selfclient: &mut AsyncClient = unsafe {mem::transmute(context)};
        selfclient.action_result = ActionResult::Ok;
        selfclient.barrier.wait();
        selfclient.random = 69;
    }

    extern "C" fn connect_failed(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_failureData) -> () {
        println!("connect failed");
        assert!(!context.is_null());
        let selfclient : &mut AsyncClient = unsafe {mem::transmute(context)};
        if response.is_null() {
            selfclient.action_result = ActionResult::Err(MqttError::Bad);
        } else {
            let resp : &mut ffiasync::MQTTAsync_failureData = unsafe {mem::transmute(response)};
            selfclient.action_result = ActionResult::Err(MqttError::Code(resp.code));
        }
        selfclient.barrier.wait();
    }

    extern "C" fn disconnected(context: *mut c_void, cause: *mut c_char) -> () {
        println!("disconnected");
        assert!(!context.is_null());
    }

    pub fn is_connected(&mut self) -> bool {
        let mut ret = unsafe {
            ffiasync::MQTTAsync_isConnected(self.handle)
        };

        match ret {
            1 => true,
            _ => false,
        }
    }

    pub fn send(&mut self, data: &[u8], topic: &str, qos: Qos) -> Result<(), MqttError>{
        let mut responseoption = ffiasync::MQTTAsync_responseOptions {
            struct_id       : ['M' as i8, 'Q' as i8, 'T' as i8, 'R' as i8],
            struct_version  : 0,
            onSuccess       : Some(Self::send_succeeded),
            onFailure       : Some(Self::send_failed),
            context         : self.context(),
            token           : 0,
        };

        let mut message = ffiasync::MQTTAsync_message {
            struct_id       : ['M' as i8, 'Q' as i8, 'T' as i8, 'M' as i8],
            struct_version  : 0,
            payloadlen      : data.len() as i32,
            payload         : unsafe {mem::transmute::<&u8, *mut c_void>(&data[0])},
            qos             : qos as c_int,
            retained        : 0,
            dup             : 0,
            msgid           : 0,
        };

        let c_topic        = CString::new(topic).unwrap();
        let array_topic    = c_topic.as_bytes_with_nul();
        self.action_result = ActionResult::None;

        let mut error = unsafe {
            ffiasync::MQTTAsync_sendMessage(self.handle,
                                            mem::transmute::<&u8, *const c_char>(&array_topic[0]),
                                            &mut message,
                                            &mut responseoption)
        };
        if error == 0 {
            self.barrier.wait();
            match self.action_result {
                ActionResult::None   => unreachable!(),
                ActionResult::Ok     => Ok(()),
                ActionResult::Err(x) => Err(x)
            }
        } else { Err(MqttError::Code(error)) }
    }

    extern "C" fn send_succeeded(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_successData) -> () {
        println!("send succeeded");
        assert!(!context.is_null());
        let selfclient: &mut AsyncClient = unsafe {mem::transmute(context)};
        selfclient.action_result = ActionResult::Ok;
        selfclient.barrier.wait();
    }

    extern "C" fn send_failed(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_failureData) -> () {
        println!("send failed");
        assert!(!context.is_null());
        let selfclient : &mut AsyncClient                     = unsafe {mem::transmute(context)};
        if response.is_null() {
            selfclient.action_result = ActionResult::Err(MqttError::Bad);
        } else {
            let resp : &mut ffiasync::MQTTAsync_failureData = unsafe {mem::transmute(response)};
            selfclient.action_result = ActionResult::Err(MqttError::Code(resp.code));
        }
        selfclient.barrier.wait();
    }

    pub fn subscribe(&mut self, topic: &str, qos: Qos) -> Result<(), MqttError> {
        let mut responseoption = ffiasync::MQTTAsync_responseOptions {
            struct_id       : ['M' as i8, 'Q' as i8, 'T' as i8, 'R' as i8],
            struct_version  : 0,
            onSuccess       : Some(Self::subscribe_succeeded),
            onFailure       : Some(Self::subscribe_failed),
            context         : self.context(),
            token           : 0,
        };

        let c_topic     = CString::new(topic).unwrap();
        let array_topic = c_topic.as_bytes_with_nul();

        let c_qos: i32 = match qos {
            Qos::FireAndForget  => 0,
            Qos::AtLeastOnce    => 1,
            Qos::OnceAndOneOnly => 2,
        };

        let mut error = unsafe {
            ffiasync::MQTTAsync_subscribe(self.handle,
                                          mem::transmute::<&u8, *const c_char>(&array_topic[0]),
                                          c_qos,
                                          &mut responseoption)
        };

        match error {
            0   => Ok(()),
            err => Err(MqttError::Code(err)),
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
        let topic   = String::from_utf8(c_topic.to_vec()).unwrap();
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

    pub keep_alive_interval : i32,
    pub cleansession        : i32,
    pub max_in_flight       : i32,
    pub connect_timeout     : i32,
    pub retry_interval      : i32,
}

impl AsyncConnectOptions {

    pub fn new() -> AsyncConnectOptions {
        let ffioptions = ffiasync::MQTTAsync_connectOptions {
            struct_id           : ['M' as i8, 'Q' as i8, 'T' as i8, 'C' as i8],
            struct_version      : 3,
            keepAliveInterval   : 60,
            cleansession        : 1,
            maxInflight         : 10,
            will                : ptr::null_mut(),
            username            : ptr::null_mut(),
            password            : ptr::null_mut(),
            connectTimeout      : 30,
            retryInterval       : 0,
            ssl                 : ptr::null_mut(),
            onSuccess           : None,
            onFailure           : None,
            context             : ptr::null_mut(),
            serverURIcount      : 0,
            serverURIs          : ptr::null_mut(),
            MQTTVersion         : 0,
        };

        let options = AsyncConnectOptions {
            options             : ffioptions,

            keep_alive_interval : 20,
            cleansession        : 1,
            max_in_flight       : 10,
            connect_timeout     : 30,
            retry_interval      : 0,
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
            struct_id       : ['M' as i8, 'Q' as i8, 'T' as i8, 'D' as i8],
            struct_version  : 0,
            timeout         : 0,
            onSuccess       : ptr::null_mut(),
            onFailure       : ptr::null_mut(),
            context         : ptr::null_mut(),
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
            struct_id       : ['M' as i8, 'Q' as i8, 'T' as i8, 'M' as i8],
            struct_version  : 0,
            payloadlen      : 0,
            payload         : ptr::null_mut(),
            qos             : 0,
            retained        : 0,
            dup             : 0,
            msgid           : 0,
        };

        message
    }
}
