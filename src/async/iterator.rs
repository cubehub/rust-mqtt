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

use time;
use std::sync::{Arc, Mutex, Condvar};
use super::Message;


pub struct AsyncClientIntoIterator {
    messages   : Arc<(Mutex<Vec<Message>>, Condvar)>,
    timeout_ms : Option<u32>,
}

impl AsyncClientIntoIterator {
    pub fn new(messages: Arc<(Mutex<Vec<Message>>, Condvar)>, timeout_ms: Option<u32>) -> Self {
        AsyncClientIntoIterator{ messages   : messages,
                                 timeout_ms : timeout_ms
        }
    }
}

impl Iterator for AsyncClientIntoIterator {
    type Item = Message;

    fn next(&mut self) -> Option<Message> {
        let &(ref msglock, ref cvar) = &*self.messages;

        if self.timeout_ms.is_some() {
            // non-blocking
            let deadline = time::now()+time::Duration::milliseconds(self.timeout_ms.unwrap() as i64);
            let mut messages = msglock.lock().unwrap();
            let mut wait_duration;
            loop {
                if messages.len() > 0 {
                    return Some(messages.remove(0))
                }
                wait_duration = (deadline-time::now()).num_milliseconds();
                if wait_duration <= 0 {
                    // timeout before condvar wait
                    return None
                }

                let (msgs, time_left) = cvar.wait_timeout_ms(messages, wait_duration as u32).unwrap();
                if !time_left {
                    // timeout - we did not get notification
                    return None
                }
                else {
                    messages = msgs;
                }
            }
        }
        else {
            // blocking
            let mut messages = msglock.lock().unwrap();
            loop {
                if messages.len() > 0 {
                    return Some(messages.remove(0))
                }
                messages = cvar.wait(messages).unwrap();
            }
        }
    }
}
