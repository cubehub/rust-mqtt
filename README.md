#rust-mqtt
This library is a Rust wrapper around [paho.mqtt.c lib](https://github.com/eclipse/paho.mqtt.c).
Currently only async lib is implemented.

## Dependencies

Build [paho.mqtt.c lib](https://github.com/eclipse/paho.mqtt.c)

    git clone https://github.com/eclipse/paho.mqtt.c.git
    cd paho.mqtt.c
    git checkout develop
    make
    sudo make install

Mac OS X

Rust-mqtt links to `libpaho-mqtt3a`. Mac OS X is not able find it from original files therefore create symlinks:

    ln -s /usr/local/lib/libpaho-mqtt3a.so.1.0 /usr/local/lib/libpaho-mqtt3a.dylib
    ln -s /usr/local/lib/libpaho-mqtt3a.so.1.0 /usr/local/lib/libpaho-mqtt3a.so.1

## Usage
Put this in your `Cargo.toml`:

```toml
[dependencies.mqtt]
git = "https://git@github.com:cubehub/rust-mqtt.git"
```

And this in your crate root:

```rust
extern crate mqtt;
```

## Examples

Install and start [mosquitto](http://mosquitto.org) broker.

Mac OS X

    brew install mosquitto
    /usr/local/sbin/mosquitto

Ubuntu

    sudo apt-get install mosquitto mosquitto-clients
    mosquitto

Start rust-mqtt [example](https://github.com/cubehub/rust-mqtt/blob/master/examples/sendreceive.rs):

    cargo run --example sendreceive


## For rust-mqtt developers

Tool called rust-bindgen is used to generate Rust functions from C files.

Mac OS X:

    echo export DYLD_LIBRARY_PATH=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/:$DYLD_LIBRARY_PATH >> ~/.profile

Build [rust-bindgen](https://github.com/crabtw/rust-bindgen)

    git clone https://github.com/crabtw/rust-bindgen.git
    cd rust-bindgen
    cargo build

Generate Rust bindings

    ./target/debug/bindgen -l libmqttv3a -match MQTTAsync.h -match MQTTClientPersistence.h -o ~/Development/rust-mqtt/src/ffimqttasync.rs  ~/Downloads/paho.mqtt.c/src/MQTTAsync.h

Notice that there are some issues with rust-bindgen generated code for callbacks. Therefore some manual modifications must be made to ffimqttasync.rs. Here is an example how to do it correctly:

* commit: [fix ffimqttasync callbacks, rust-bindgen got them wrong](https://github.com/cubehub/rust-mqtt/commit/b3172439b11a4faff66750ece80371a90c34a0f9)
