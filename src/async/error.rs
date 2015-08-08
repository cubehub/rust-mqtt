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

use std::fmt;
use std::error::Error;


#[derive(Debug, Clone)]
pub enum MqttError {
    Create(i32),
    Connect(ConnectError),
    Subscribe(CommandError),
    Send(CommandError),
}
impl fmt::Display for MqttError {
    fn fmt(&self, f:&mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            MqttError::Create(ref x)    => fmt::Display::fmt(&format!("MqttError::Create({:?})", x), f),
            MqttError::Connect(ref x)   => fmt::Display::fmt(&format!("MqttError::Connect({:?})", x), f),
            MqttError::Subscribe(ref x) => fmt::Display::fmt(&format!("MqttError::Subscribe({:?})", x), f),
            MqttError::Send(ref x)      => fmt::Display::fmt(&format!("MqttError::Send({:?})", x), f),
        }
    }
}
impl Error for MqttError {
    fn description(&self) -> &str {
        match *self {
            MqttError::Create(_)    => "Mqtt creation failed",
            MqttError::Connect(_)   => "Mqtt connect failed",
            MqttError::Subscribe(_) => "Mqtt subscribe failed",
            MqttError::Send(_)      => "Mqtt send failed",
        }
    }
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
    pub fn from_int(i:i32) -> Self {
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

pub enum CallbackError {
    Response(i32),
    NullPtr
}
