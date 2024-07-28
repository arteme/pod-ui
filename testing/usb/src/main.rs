use std::io::ErrorKind;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvError;
use std::thread;
use std::time::Duration;
use usb_gadget::{Class, Config, default_udc, Gadget, Id, Strings};
use usb_gadget::function::custom::{Custom, Endpoint, EndpointDirection, Interface};
use bytes::BytesMut;

const FAMILY: u16 = 0x0000;
const MEMBER: u16 = 0x0600;

fn reply(req: &[u8]) -> Option<Vec<u8>> {

    match req {
        &[0xf0, 0x7e, channel, 0x06, 0x01, 0xf7] => {
            // UDI
            // channel can vary, but in practice, this will always ne 0x7f
            let family = u16::to_le_bytes(FAMILY);
            let member = u16::to_le_bytes(MEMBER);
            let ver = format!("{:4}", "1.01").into_bytes();
            Some([0xf0, 0x7e, channel, 0x06, 0x02, 0x00, 0x01, 0x0c, family[0], family[1], member[0], member[1],
                  ver[0], ver[1], ver[2], ver[3], 0xf7].to_vec())
        }
        &_ => { None }
    }
}

fn main() {
    usb_gadget::remove_all().expect("cannot remove all gadgets");

    let (mut cmd_tx, cmd_rx) = mpsc::channel::<Vec<u8>>();

    let (mut ep1_rx, ep1_dir) = EndpointDirection::host_to_device();
    let (mut ep2_tx, ep2_dir) = EndpointDirection::device_to_host();

    let (mut custom, handle) = Custom::builder()
        .with_interface(
            Interface::new(Class::vendor_specific(1, 2), "custom interface")
                .with_endpoint(Endpoint::bulk(ep1_dir))
                .with_endpoint(Endpoint::bulk(ep2_dir)),
        )
        .build();

    let udc = default_udc().expect("cannot get UDC");
    let reg = Gadget::new(
        Class::new(255, 255, 3),
        Id::new(0x0010, 0x0001),
        Strings::new("POD-UI", "testing device", "serial_number"),
    )
    .with_config(Config::new("config").with_function(handle))
    .bind(&udc)
    .expect("cannot bind to UDC");

    println!("Custom function at {}", custom.status().unwrap().path().unwrap().display());
    println!();

    let ep1_control = ep1_rx.control().unwrap();
    println!("ep1 unclaimed: {:?}", ep1_control.unclaimed_fifo());
    println!("ep1 real address: {}", ep1_control.real_address().unwrap());
    println!("ep1 descriptor: {:?}", ep1_control.descriptor().unwrap());
    println!();

    let ep2_control = ep2_tx.control().unwrap();
    println!("ep2 unclaimed: {:?}", ep2_control.unclaimed_fifo());
    println!("ep2 real address: {}", ep2_control.real_address().unwrap());
    println!("ep2 descriptor: {:?}", ep2_control.descriptor().unwrap());
    println!();

    let stop = Arc::new(AtomicBool::new(false));

    thread::scope(|s| {
        thread::Builder::new()
            .name("rx".into())
            .spawn_scoped(s, move || {
                let size = ep1_rx.max_packet_size().unwrap();
                while !stop.load(Ordering::Relaxed) {
                    let res = ep1_rx
                        .recv_timeout(BytesMut::with_capacity(size), Duration::from_secs(1));
                    let data = match res {
                        Ok(v) => { v }
                        Err(e) if e.raw_os_error() == Some(108) => {
                            // Ignore this error -- they seem to come while in process of data transfer:
                            // Os { code: 108, kind: Uncategorized, message: "Cannot send after transport endpoint shutdown" }
                            continue;
                        }
                        Err(e) => {
                            println!("RX error: {e:?}");
                            continue
                        }
                    };
                    match data {
                        Some(data) => {
                            let d = data.as_ref();
                            println!("<< {:02x?} len={}", d, d.len());
                            if let Some(rep) = reply(d) {
                                thread::sleep(Duration::from_millis(500));
                                cmd_tx.send(rep).ok();
                            }
                        }
                        None => {
                            // empty
                        }
                    }
                }
            }).ok();

        thread::Builder::new()
            .name("tx".into())
            .spawn_scoped(s, move || {
                let size = ep2_tx.max_packet_size().unwrap();
                loop {
                //while !stop.load(Ordering::Relaxed) {
                    let data = match cmd_rx.recv() {
                        Ok(data) => { data }
                        Err(e) => {
                            println!("Command rx error: {}", e);
                            continue;
                        }
                    };

                    match ep2_tx.send_timeout(data.clone().into(), Duration::from_secs(1)) {
                        Ok(()) => {
                            println!(">> {:02x?} len={}", &data, data.len());
                        }
                        Err(err) if err.kind() == ErrorKind::TimedOut => println!("send timeout"),
                        Err(err) => panic!("send failed: {err}"),
                    }
                }
            }).ok();

        thread::Builder::new()
            .name("control".into())
            .spawn_scoped(s, || {
                //let mut ctrl_data = Vec::new();

                loop {
                //while !stop.load(Ordering::Relaxed) {
                    let data =
                        custom.event_timeout(Duration::from_secs(1))
                            .expect("event failed");
                    if let Some(event) = data {
                        println!("Event: {event:?}");
                        /*
                        match event {
                            Event::SetupHostToDevice(req) => {
                                if req.ctrl_req().request == 255 {
                                    println!("Stopping");
                                    stop.store(true, Ordering::Relaxed);
                                }
                                ctrl_data = req.recv_all().unwrap();
                                println!("Control data: {ctrl_data:x?}");
                            }
                            Event::SetupDeviceToHost(req) => {
                                println!("Replying with data");
                                req.send(&ctrl_data).unwrap();
                            }
                            _ => (),
                        }
                         */

                    }
                }
            }).ok();
    });

    thread::sleep(Duration::from_secs(1));

    println!("Unregistering");
    reg.remove().unwrap();
}
