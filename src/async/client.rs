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
use std::slice;
use std::sync::{Barrier, Mutex, Arc};

use super::Message;
use super::options::{PersistenceType, Qos, AsyncConnectOptions, AsyncDisconnectOptions};
use super::error::{MqttError, CommandError, ConnectError, ConnectErrReturnCode, DisconnectError, DisconnectErrReturnCode, CallbackError};
use super::iterator::AsyncClientIntoIterator;


pub struct AsyncClient {
    inner: Box<ImmovableClient>
}

impl AsyncClient {
    pub fn new(address: &str, clientid: &str, persistence: PersistenceType) -> Result<Self, MqttError> {
        let mut ac = AsyncClient {
            inner: Box::new(ImmovableClient::new(address, clientid, persistence))
        };
        try!(ac.inner.create());
        Ok(ac)
    }
    pub fn connect(&mut self, options: &AsyncConnectOptions) -> Result<(), MqttError> {
        self.inner.connect(options)
    }
    pub fn disconnect(&mut self, options: &AsyncDisconnectOptions) -> Result<(), MqttError> {
        self.inner.disconnect(options)
    }
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }
    pub fn send(&mut self, data: &[u8], topic: &str, qos: Qos) -> Result<(), MqttError> {
        self.inner.send(data, topic, qos)
    }
    pub fn subscribe(&mut self, topic: &str, qos: Qos) -> Result<(), MqttError> {
        self.inner.subscribe(topic, qos)
    }
    pub fn messages(&mut self) -> AsyncClientIntoIterator {
        AsyncClientIntoIterator::new(self.inner.messages.clone())
    }
}


struct ImmovableClient {
    c_url               : CString,
    c_clientid          : CString,
    handle              : ffiasync::MQTTAsync,
    persistence_context : c_void,
    persistence         : PersistenceType,

    barrier       : Barrier,
    action_result : Option<Result<(), CallbackError>>,
    pub messages  : Arc<Mutex<Vec<Message>>>,
}
impl ImmovableClient {
    fn context(&mut self) -> *mut c_void {
        self as *mut _ as *mut c_void
    }

    pub fn new(address: &str, clientid: &str, persistence: PersistenceType) -> Self {
        ImmovableClient {
                    c_url               : CString::new(address).unwrap(),
                    c_clientid          : CString::new(clientid).unwrap(),
                    handle              : unsafe{mem::zeroed()},
                    persistence_context : unsafe{mem::zeroed()},
                    persistence         : persistence,

                    barrier         : Barrier::new(2),
                    action_result   : None,
                    messages        : Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn create(&mut self) -> Result<(), MqttError> {
        let array_url      = self.c_url.as_bytes_with_nul();
        let array_clientid = self.c_clientid.as_bytes_with_nul();
        let error = unsafe {
            ffiasync::MQTTAsync_create(&mut self.handle,
                                       mem::transmute::<&u8, *const c_char>(&array_url[0]),
                                       mem::transmute::<&u8, *const c_char>(&array_clientid[0]),
                                       self.persistence as i32,
                                       &mut self.persistence_context)
        };
        match error {
            0   => { Ok(())},
            err => Err(MqttError::Create(err))
        }
    }

    pub fn connect(&mut self, options: &AsyncConnectOptions) -> Result<(), MqttError> {
        debug!("connect..");
        unsafe {
            ffiasync::MQTTAsync_setCallbacks(self.handle,
                                             self.context(),
                                             Some(Self::disconnected),
                                             Some(Self::received),
                                             None);
        }

        let mut async_opts = ffiasync::MQTTAsync_connectOptions::new();

        // fill in FFI private struct
        async_opts.keepAliveInterval = options.keep_alive_interval;
        async_opts.cleansession      = options.cleansession;
        async_opts.maxInflight       = options.max_in_flight;
        async_opts.connectTimeout    = options.connect_timeout;
        async_opts.retryInterval     = options.retry_interval;

        // register callbacks
        async_opts.context   = self.context();
        async_opts.onSuccess = Some(Self::action_succeeded);
        async_opts.onFailure = Some(Self::action_failed);

        self.action_result = None;
        let error = unsafe {
            ffiasync::MQTTAsync_connect(self.handle, &async_opts)
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

    pub fn disconnect(&mut self, options: &AsyncDisconnectOptions) -> Result<(), MqttError> {
        debug!("disconnect..");
        // client must be already conneced to do disconnect
        if !self.is_connected() {
            Ok(())
        }
        else {
            let mut async_opts = ffiasync::MQTTAsync_disconnectOptions::new();

            // fill in FFI private struct
            async_opts.timeout = options.timeout;

            // register callbacks
            async_opts.context   = self.context();
            async_opts.onSuccess = Some(Self::action_succeeded);
            async_opts.onFailure = Some(Self::action_failed);

            self.action_result = None;
            let error = unsafe {
                ffiasync::MQTTAsync_disconnect(self.handle, &async_opts)
            };
            if error == 0 {
                self.barrier.wait();
                match (self.is_connected(), &self.action_result) {
                    (false, _                                     ) => Ok(()),
                    (_,     &None                                 ) => unreachable!(),  // barrier should ensure we have something
                    (_,     &Some(Ok(()))                         ) => unreachable!(),  // callback and is_connected() don't agree?
                    (_,     &Some(Err(CallbackError::Response(r)))) => Err(MqttError::Disconnect(DisconnectError::CallbackResponse(r))),
                    (_,     &Some(Err(CallbackError::NullPtr))    ) => Err(MqttError::Disconnect(DisconnectError::CallbackNullPtr)),
                }
            } else { Err(MqttError::Disconnect(DisconnectError::ReturnCode(DisconnectErrReturnCode::from_int(error)))) }
        }
    }

    #[allow(unused_variables)]
    extern "C" fn disconnected(context: *mut c_void, cause: *mut c_char) -> () {
        warn!("disconnected");
        assert!(!context.is_null());
    }

    pub fn is_connected(&self) -> bool {
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
        let selfclient: &mut ImmovableClient = unsafe {mem::transmute(context)};
        selfclient.action_result = Some(Ok(()));
        selfclient.barrier.wait();
    }

    extern "C" fn action_failed(context: *mut ::libc::c_void, response: *mut ffiasync::MQTTAsync_failureData) -> () {
        debug!("failure callback");
        assert!(!context.is_null());
        let selfclient : &mut ImmovableClient = unsafe {mem::transmute(context)};
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

        let payload = match transmessage.payloadlen {
            0  => None,
            _ => {
                let payload_slice: &[u8] = unsafe {
                        slice::from_raw_parts(transmessage.payload as *mut u8, transmessage.payloadlen as usize)
                };
                Some(payload_slice.to_vec())
            }
        };

        assert!(!context.is_null());
        let selfclient : &mut ImmovableClient = unsafe {mem::transmute(context)};

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
            payload   : payload,
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

}
impl Drop for ImmovableClient {
    fn drop(&mut self) {
        unsafe{ffiasync::MQTTAsync_destroy(&mut self.handle)};
    }
}
