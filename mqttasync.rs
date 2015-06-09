
use mqtt::ffimqttasync;

use std::mem;
use libc::{c_char, c_void};
use std::ffi::CString;

#[allow(non_camel_case_types)]
pub enum PersistentType {
    PERSISTENCE_DEFAULT = 0,
    PERSISTENCE_NONE = 1,
    PERSISTENCE_USER = 2,
}

pub struct ClientAsync {
    client: ffimqttasync::MQTTAsync,
}

impl ClientAsync {

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
