/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2015 Andres Vahter (andres.vahter@gmail.com)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use ffiasync;

use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;
use std::slice;
use std::sync::{Barrier, Mutex};

pub enum PersistenceType {
    Default = 0,
    Nothing = 1,
    User    = 2,
}

#[derive(Debug)]
pub enum Qos {
    FireAndForget  = 0,
    AtLeastOnce    = 1,
    OnceAndOneOnly = 2,
}
impl Qos {
    fn from_int(i:i32) -> Self {
        match i {
            0 => Qos::FireAndForget,
            1 => Qos::AtLeastOnce,
            2 => Qos::OnceAndOneOnly,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct Message {
    pub topic     : String,
    pub payload   : Vec<u8>,
    pub qos       : Qos,
    pub retained  : bool,
    pub duplicate : bool,
}

#[derive(Debug, Clone)]
pub enum MqttError {
    Create(i32),
    Connect(ConnectError),
    Subscribe(CommandError),
    Send(CommandError),
}

#[derive(Debug, Clone)]
pub enum CommandError {
    ReturnCode(i32),
    CallbackResponse(i32),
    CallbackNullPtr
}

#[derive(Debug, Clone)]
pub enum ConnectError {
    ReturnCode(ConnectErrReturnCode),
    CallbackResponse(i32),
    CallbackNullPtr
}

#[derive(Debug, Clone)]
pub enum ConnectErrReturnCode {
    UnacceptableProtocol  = 1,
    IdentifierRejected    = 2,
    ServerUnavailable     = 3,
    BadUsernameOrPassword = 4,
    NotAuthorized         = 5,
    Reserved              = 6,
}
impl ConnectErrReturnCode {
    fn from_int(i:i32) -> Self {
        match i {
            1 => ConnectErrReturnCode::UnacceptableProtocol,
            2 => ConnectErrReturnCode::IdentifierRejected,
            3 => ConnectErrReturnCode::ServerUnavailable,
            4 => ConnectErrReturnCode::BadUsernameOrPassword,
            5 => ConnectErrReturnCode::NotAuthorized,
            6 => ConnectErrReturnCode::Reserved,
            _ => unreachable!()
        }
    }
}

enum CallbackError {
    Response(i32),
    NullPtr
}

pub struct AsyncClient {
    handle        : ffiasync::MQTTAsync,
    barrier       : Barrier,
    action_result : Option<Result<(), CallbackError>>,
    messages      : Mutex<Vec<Message>>,
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

        let error = unsafe {
            ffiasync::MQTTAsync_create(&mut handle,
                                       mem::transmute::<&u8, *const c_char>(&array_url[0]),
                                       mem::transmute::<&u8, *const c_char>(&array_clientid[0]),
                                       persistence as i32,
                                       &mut persistence_context)
        };

        match error {
            0   => { Ok( AsyncClient {
                        handle          : handle,
                        barrier         : Barrier::new(2),
                        action_result   : None,
                        messages        : Mutex::new(Vec::new()),
                    })},
            err => Err(MqttError::Create(err))
        }
    }

    pub fn connect(&mut self, options: &mut AsyncConnectOptions) -> Result<(), MqttError> {
        debug!("connect..");
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
        options.options.onSuccess = Some(Self::action_succeeded);
        options.options.onFailure = Some(Self::action_failed);

        self.action_result = None;
        let error = unsafe {
            ffiasync::MQTTAsync_connect(self.handle, &options.options)
        };
        if error == 0 {
            self.barrier.wait();
            match (self.is_connected(), &self.action_result) {
                (true,  _                                     ) => Ok(()),
                (false, &None                                 ) => unreachable!(),  // barrier should ensure we have something
                (false, &Some(Ok(()))                         ) => unreachable!(),  // callback and is_connected() don't agree?
                (false, &Some(Err(CallbackError::Response(r)))) => Err(MqttError::Connect(ConnectError::CallbackResponse(r))),
                (false, &Some(Err(CallbackError::NullPtr))    ) => Err(MqttError::Connect(ConnectError::CallbackNullPtr)),
            }
        } else { Err(MqttError::Connect(ConnectError::ReturnCode(ConnectErrReturnCode::from_int(error)))) }
    }

    #[allow(unused_variables)]
    extern "C" fn disconnected(context: *mut c_void, cause: *mut c_char) -> () {
        warn!("disconnected");
        assert!(!context.is_null());
    }

    pub fn is_connected(&mut self) -> bool {
        let ret = unsafe {
            ffiasync::MQTTAsync_isConnected(self.handle)
        };

        match ret {
            1 => true,
            _ => false,
        }
    }

    pub fn send(&mut self, data: &[u8], topic: &str, qos: Qos) -> Result<(), MqttError> {
        debug!("send..");
        let mut responseoption = ffiasync::MQTTAsync_responseOptions {
            struct_id       : ['M' as i8, 'Q' as i8, 'T' as i8, 'R' as i8],
            struct_version  : 0,
            onSuccess       : Some(Self::action_succeeded),
            onFailure       : Some(Self::action_failed),
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
        self.action_result = None;

        let error = unsafe {
            ffiasync::MQTTAsync_sendMessage(self.handle,
                                            mem::transmute::<&u8, *const c_char>(&array_topic[0]),
                                            &mut message,
                                            &mut responseoption)
        };
        if error == 0 {
            self.barrier.wait();
            match (self.is_connected(), &self.action_result) {
                (true,  _                                     ) => Ok(()),
                (false, &None                                 ) => unreachable!(),  // barrier should ensure we have something
                (false, &Some(Ok(()))                         ) => unreachable!(),  // callback and is_connected() don't agree?
                (false, &Some(Err(CallbackError::Response(r)))) => Err(MqttError::Send(CommandError::CallbackResponse(r))),
                (false, &Some(Err(CallbackError::NullPtr))    ) => Err(MqttError::Send(CommandError::CallbackNullPtr)),
            }
        } else { Err(MqttError::Send(CommandError::ReturnCode(error))) }
    }

    pub fn subscribe(&mut self, topic: &str, qos: Qos) -> Result<(), MqttError> {
        debug!("subscribe..");
        let mut responseoption = ffiasync::MQTTAsync_responseOptions {
            struct_id       : ['M' as i8, 'Q' as i8, 'T' as i8, 'R' as i8],
            struct_version  : 0,
            onSuccess       : Some(Self::action_succeeded),
            onFailure       : Some(Self::action_failed),
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
        self.action_result = None;

        let error = unsafe {
            ffiasync::MQTTAsync_subscribe(self.handle,
                                          mem::transmute::<&u8, *const c_char>(&array_topic[0]),
                                          c_qos,
                                          &mut responseoption)
        };

        if error == 0 {
            self.barrier.wait();
            match (self.is_connected(), &self.action_result) {
                (true,  _                                     ) => Ok(()),
                (false, &None                                 ) => unreachable!(),  // barrier should ensure we have something
                (false, &Some(Ok(()))                         ) => unreachable!(),  // callback and is_connected() don't agree?
                (false, &Some(Err(CallbackError::Response(r)))) => Err(MqttError::Subscribe(CommandError::CallbackResponse(r))),
                (false, &Some(Err(CallbackError::NullPtr))    ) => Err(MqttError::Subscribe(CommandError::CallbackNullPtr)),
            }
        } else { Err(MqttError::Subscribe(CommandError::ReturnCode(error))) }
    }

    #[allow(unused_variables)]
    extern "C" fn action_succeeded(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_successData) -> () {
        debug!("success callback");
        assert!(!context.is_null());
        let selfclient: &mut AsyncClient = unsafe {mem::transmute(context)};
        selfclient.action_result = Some(Ok(()));
        selfclient.barrier.wait();
    }

    extern "C" fn action_failed(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_failureData) -> () {
        debug!("failure callback");
        assert!(!context.is_null());
        let selfclient : &mut AsyncClient = unsafe {mem::transmute(context)};
        if response.is_null() {
            selfclient.action_result = Some(Err(CallbackError::NullPtr));
        } else {
            let resp : &mut ffiasync::MQTTAsync_failureData = unsafe {mem::transmute(response)};
            selfclient.action_result = Some(Err(CallbackError::Response(resp.code)));
        }
        selfclient.barrier.wait();
    }

    extern "C" fn received(context: *mut ::libc::c_void, topic_name: *mut ::libc::c_char, topic_len: ::libc::c_int, amessage: *mut ffiasync::MQTTAsync_message) -> i32 {
        let c_topic = unsafe {CStr::from_ptr(topic_name).to_bytes()};
        let topic   = String::from_utf8(c_topic.to_vec()).unwrap();
        assert_eq!(topic.len(), topic_len as usize);

        assert!(!amessage.is_null());
        let transmessage: &mut ffiasync::MQTTAsync_message = unsafe {mem::transmute(amessage)};
        let payload: &[u8] = unsafe {
                slice::from_raw_parts(transmessage.payload as *mut u8, transmessage.payloadlen as usize)
        };

        assert!(!context.is_null());
        let selfclient : &mut AsyncClient = unsafe {mem::transmute(context)};

        let qos = Qos::from_int(transmessage.qos);

        let retained: bool = match transmessage.retained {
            0 => false,
            1 => true,
            _ => unreachable!(),
        };

        let duplicate: bool = match transmessage.dup {
            0 => false,
            1 => true,
            _ => unreachable!(),
        };


        let msg = Message {
            topic     : topic,
            payload   : payload.to_vec(),
            qos       : qos,
            retained  : retained,
            duplicate : duplicate,
        };

        let mut messages = selfclient.messages.lock().unwrap();
        messages.push(msg);

        let mut msg = amessage;
        unsafe{ffiasync::MQTTAsync_freeMessage(&mut msg)};
        unsafe{ffiasync::MQTTAsync_free(mem::transmute(topic_name))};
        1
    }

    pub fn messages(&mut self) -> AsyncClientIntoIterator {
        AsyncClientIntoIterator {
            client: self,
        }
    }
}

pub struct AsyncClientIntoIterator<'a> {
    client: &'a mut AsyncClient,
}

impl <'a>Iterator for AsyncClientIntoIterator<'a> {
    type Item = Message;
    fn next(&mut self) -> Option<Message> {
        let mut messages = self.client.messages.lock().unwrap();
        if messages.len() > 0 {
            Some(messages.remove(0))
        }
        else {
            None
        }
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

// it is going to be used in the future
// just silence warning for now
#[allow(dead_code)]
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
