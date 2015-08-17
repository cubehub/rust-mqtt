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
use std::ptr;

#[derive(Debug, Copy, Clone)]
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
    pub fn from_int(i:i32) -> Self {
        match i {
            0 => Qos::FireAndForget,
            1 => Qos::AtLeastOnce,
            2 => Qos::OnceAndOneOnly,
            _ => unreachable!(),
        }
    }
}

impl ffiasync::MQTTAsync_connectOptions {
    pub fn new() -> Self {
        ffiasync::MQTTAsync_connectOptions {
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
        }
    }
}

pub struct AsyncConnectOptions {
    pub keep_alive_interval : i32,
    pub cleansession        : i32,
    pub max_in_flight       : i32,
    pub connect_timeout     : i32,
    pub retry_interval      : i32,
}
impl AsyncConnectOptions {
    pub fn new() -> AsyncConnectOptions {
        AsyncConnectOptions {
            keep_alive_interval : 20,
            cleansession        : 1,
            max_in_flight       : 10,
            connect_timeout     : 30,
            retry_interval      : 0,
        }
    }
}

// it is going to be used in the future
// just silence warning for now
#[allow(dead_code)]
impl ffiasync::MQTTAsync_disconnectOptions {
    pub fn new() -> Self {
        ffiasync::MQTTAsync_disconnectOptions {
            struct_id       : ['M' as i8, 'Q' as i8, 'T' as i8, 'D' as i8],
            struct_version  : 0,
            timeout         : 0,
            onSuccess       : ptr::null_mut(),
            onFailure       : ptr::null_mut(),
            context         : ptr::null_mut(),
        }
    }
}

#[allow(dead_code)]
pub struct AsyncDisconnectOptions;
impl AsyncDisconnectOptions {
    #[allow(dead_code)]
    pub fn new() -> Self {
        unimplemented!()
    }
}
