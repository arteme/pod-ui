use anyhow::*;
use core::result::Result::Ok;
use std::ffi::{c_int, c_uint};
use std::mem::align_of;
use std::ptr::{NonNull, null_mut};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use log::{debug, error, info};
use rusb::{Context, Hotplug, UsbContext};
use rusb::constants::{LIBUSB_ENDPOINT_DIR_MASK, LIBUSB_ENDPOINT_IN, LIBUSB_ENDPOINT_OUT, LIBUSB_TRANSFER_TYPE_BULK};
use rusb::ffi::{libusb_alloc_transfer, libusb_cancel_transfer, libusb_free_transfer, libusb_submit_transfer, libusb_transfer};
use crate::check;
use crate::devices::find_device;
use crate::util::usb_address_string;

pub type Device = rusb::Device<Context>;
pub type DeviceHandle = rusb::DeviceHandle<Context>;

#[derive(Clone, Debug)]
pub struct ListedDevice {
    pub vid: u16,
    pub pid: u16,
    pub bus: u8,
    pub address: u8,
}

pub struct Usb {
    ctx: Context,
    running: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>
}

impl Usb {
    pub fn new() -> Result<Self> {
        let ctx = libusb::new_ctx(rusb::constants::LIBUSB_LOG_LEVEL_INFO)?;
        let running = Arc::new(AtomicBool::new(true));

        let thread = {
            let (ctx, run) = (ctx.clone(), Arc::clone(&running));
            Some(thread::spawn(move || Self::event_thread(ctx, run)))
        };
        Ok(Self { ctx, running, thread })
    }

    pub fn list_devices(&self) -> Result<Vec<ListedDevice>> {
        let devices = self.ctx.devices()?;
        let devices = devices.iter()
            .flat_map(|dev| {
                let Ok(desc) = dev.device_descriptor() else {
                    return None;
                };
                let vid = desc.vendor_id();
                let pid = desc.product_id();
                if find_device(vid, pid).is_some() {
                    let bus = dev.bus_number();
                    let address = dev.address();

                    Some(ListedDevice { vid, pid, bus, address })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(devices)
    }

    pub fn close(&mut self) {
        self.running.store(false, Ordering::Release);
        self.ctx.interrupt_handle_events();
        self.thread.take().map(thread::JoinHandle::join);
    }

    pub fn open(&self, vid: u16, pid: u16, bus: u8, address: u8) -> Result<DeviceHandle> {
        let addr_str = usb_address_string(bus, address);
        info!("Opening {:04X}:{:04X} at {}", vid, pid, addr_str);

        for dev in self.ctx.devices()?.iter() {
            if dev.bus_number() != bus || dev.address() != address { continue }
            return dev.open().map_err(|e| {
                anyhow!("Failed to open USB device {:04X}:{:04X} at {}: {}", vid, pid, addr_str, e)
            });
        }

        bail!("USB device not found!");
    }

    /// Dedicated thread for async transfer and hotplug events.
    fn event_thread(ctx: Context, run: Arc<AtomicBool>) {
        debug!("USB event thread start");
        while run.load(Ordering::Acquire) {
            if let Err(e) = ctx.handle_events(None) {
                // TODO: Stop all transfers?
                error!("Event thread error: {e}");
                break;
            }
        }
        debug!("USB event thread finish");
    }
}

impl Drop for Usb {
    fn drop(&mut self) {
        self.close();
    }
}

pub enum TransferCommand {
    Resubmit,
    Drop
}

pub enum TransferStatus {
    Ok,
    Error(rusb::Error),
    Cancel
}

struct TransferInner(NonNull<libusb_transfer>);

impl TransferInner {
    pub fn new() -> Option<Self> {
        NonNull::new(unsafe { libusb_alloc_transfer(0) })
            //.map(|inner| {debug!("new inner={:?}", inner); inner})
            .map(|inner| Self(inner))
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut libusb_transfer {
        self.0.as_ptr()
    }

    /// Returns a shared reference to the `libusb_transfer` struct.
    #[inline]
    pub const fn as_ref(&self) -> &libusb_transfer {
        // SAFETY: new() ensures that inner can be converted to a reference
        unsafe { self.0.as_ref() }
    }

    /// Returns a mutable reference to the `libusb_transfer` struct.
    #[inline]
    pub fn as_mut(&mut self) -> &mut libusb_transfer {
        // SAFETY: new() ensures that inner can be converted to a reference
        unsafe { self.0.as_mut() }
    }
}

/// Async transfer
pub struct Transfer
{
    inner: TransferInner,
    handle: Arc<DeviceHandle>,
    buf: Vec<u8>,
    status: Arc<Mutex<TransferStatus>>,
    callback: Option<Box<dyn FnMut(Option<&[u8]>) -> TransferCommand + Send>>
}

/// A submitted transfer that will update the result of the
/// original [Transfer] when the submit callback is called
pub struct SubmittedTransfer {
    status: Arc<Mutex<TransferStatus>>,
    inner: *mut libusb_transfer
}

unsafe impl Sync for SubmittedTransfer {}
unsafe impl Send for SubmittedTransfer {}

impl Transfer
{
    pub fn new_bulk(handle: &Arc<DeviceHandle>, endpoint: u8, len: usize) -> Box<Self> {
        Self::new(handle, LIBUSB_TRANSFER_TYPE_BULK, endpoint, len)
    }

    pub fn new_bulk_with_data(handle: &Arc<DeviceHandle>, endpoint: u8, data: &[u8]) -> Box<Self> {
        let mut t = Self::new(handle, LIBUSB_TRANSFER_TYPE_BULK, endpoint, data.len());
        unsafe { t.buf.set_len(data.len()); }
        t.buf.copy_from_slice(data);

        t
    }

    fn new(handle: &Arc<DeviceHandle>, transfer_type: u8, endpoint: u8, len: usize) -> Box<Self> {
        let inner = TransferInner::new()
            .expect("failed to allocate libusb_transfer struct");
        assert_eq!(inner.0.as_ptr() as usize % align_of::<libusb_transfer>(), 0);
        let buf = Vec::with_capacity(len);
        let status = Arc::new(Mutex::new(TransferStatus::Ok));
        let mut transfer = Self {
            inner, buf, handle: handle.clone(),
            status, callback: None
        };

        let inner = transfer.inner.as_mut();
        inner.endpoint = endpoint;
        inner.transfer_type = transfer_type;
        inner.callback = Self::callback;

        Box::new(transfer)
    }

    pub fn set_callback<F: FnMut(Option<&[u8]>) -> TransferCommand + Send + 'static>(&mut self, cb: F) {
        self.callback = Some(Box::new(cb))
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        let inner = self.inner.as_mut();
        inner.timeout = c_uint::try_from(timeout.as_millis()).unwrap_or(c_uint::MAX);
    }

    pub fn submit(mut self: Box<Self>) -> Result<SubmittedTransfer> {
        let buf_ptr = self.buf.as_mut_ptr();
        let buf_len = match self.inner.as_ref().endpoint & LIBUSB_ENDPOINT_DIR_MASK {
            LIBUSB_ENDPOINT_OUT => self.buf.len(),
            LIBUSB_ENDPOINT_IN => self.buf.capacity(),
            _ => unreachable!(),
        };
        let dev_handle = self.handle.as_raw();
        let inner = self.inner.as_mut();

        inner.dev_handle = dev_handle;
        inner.length = c_int::try_from(buf_len).unwrap();
        inner.buffer = buf_ptr;

        let inner = self.inner.as_ptr();
        let status = self.status.clone();
        let raw = Box::into_raw(self);
        // SAFETY: inner is a valid pointer
        unsafe { (*inner).user_data = raw.cast() };

        {
            //println!("submitting transfer={raw:?} inner={inner:?}");
            let mut status = status.lock().unwrap();
            Transfer::submit_inner(raw, inner, &mut status)?;
        }
        Ok(SubmittedTransfer {
            status,
            inner
        })
    }

    fn submit_inner(raw: *mut Self, inner: *mut libusb_transfer, status: &mut TransferStatus) -> Result<()> {
        if let Err(e) = check!(libusb_submit_transfer(inner)) {
            *status = TransferStatus::Error(e);
            Self::callback_inner(raw, inner, status);
            return Err(e.into());
        }
        Ok(())
    }

    fn callback_inner(raw: *mut Self, inner: *mut libusb_transfer, status: &mut TransferStatus) {
        // SAFETY: raw is a valid reference and we have exclusive access
        let t = unsafe { &mut *raw };
        let inner_ptr = inner;
        let inner = t.inner.as_mut();

        //println!("callback transfer={raw:?} inner={inner_ptr:?}");
        let command = match (&status, t.callback.as_mut()) {
            (TransferStatus::Cancel, Some(cb)) => {
                // Transfer was cancelled
                cb(None);
                TransferCommand::Drop
            }
            (TransferStatus::Error(e), _) => {
                // Transfer submit failed
                let dir = match inner.endpoint & LIBUSB_ENDPOINT_DIR_MASK {
                    LIBUSB_ENDPOINT_OUT => "write",
                    LIBUSB_ENDPOINT_IN => "read",
                    _ => unreachable!(),
                };
                error!("Failed to submit {} transfer: {}", dir, e);
                TransferCommand::Drop
            }
            (TransferStatus::Cancel, _) => {
                // Transfer cancelled without callback
                TransferCommand::Drop
            }
            (TransferStatus::Ok, Some(cb)) => {
                // SAFETY: buffer allocated with capacity >= actual_length in submit()
                let buf = unsafe { std::slice::from_raw_parts(
                    inner.buffer,
                    inner.actual_length as usize
                ) };
                cb(Some(buf))
            }
            (TransferStatus::Ok, None) => {
                // Transfer successful, but no callback
                TransferCommand::Drop
            }
        };
        match command {
            TransferCommand::Resubmit => {
                Transfer::submit_inner(raw, inner_ptr, status).ok();
            }
            TransferCommand::Drop => {
                inner.dev_handle = null_mut();
                inner.user_data = null_mut();
                *status = TransferStatus::Cancel;
                // SAFETY: We have the only pointer to the original Transfer
                drop(unsafe { Box::from_raw(t as _) });
            }
        }

    }

    /// Handles transfer completion callback.
    extern "system" fn callback(inner: *mut libusb_transfer) {
        let r = std::panic::catch_unwind(|| {

            // SAFETY: user_data was set in submit()
            let raw: *mut Transfer = unsafe { (*inner).user_data.cast() };
            let Some(t) = (unsafe { raw.as_ref() }) else { return };

            let mut status = t.status.lock().unwrap();
            Transfer::callback_inner(raw, inner, &mut status);
        });
        if let Err(e) = r {
            eprintln!("libusb_transfer callback panic: {e:?}");
            std::process::abort();
        }
    }
}

impl SubmittedTransfer {
    pub fn cancel(&mut self) -> Result<()> {
        let mut status = self.status.lock().unwrap();
        match *status {
            TransferStatus::Ok => {
                *status = TransferStatus::Cancel;
                check!(libusb_cancel_transfer(self.inner)).map_err(|e| e.into())
            }
            _ => { Ok(()) }
        }
    }
}

impl Drop for Transfer {
    fn drop(&mut self) {
        //println!("drop inner={:?}", self.inner.as_ptr());

        // SAFETY: C API call, inner can be null
        unsafe { libusb_free_transfer(self.inner.as_ptr()) }
    }
}

pub mod libusb {
    use std::ffi::{c_char, c_int, c_void, CStr};
    use std::ptr::null_mut;
    use std::sync::Once;
    use log::{debug, error, info, trace, warn};
    use rusb::constants::*;
    use rusb::{Context, DeviceHandle, Error, UsbContext};
    use rusb::ffi::{libusb_context, libusb_set_log_cb, libusb_set_option};

    #[macro_export]
    macro_rules! check {
        ($x:expr) => {
            // SAFETY: C API call
            match unsafe { $x } {
                rusb::constants::LIBUSB_SUCCESS => {
                    Ok(())
                },
                e => {
                    Err($crate::usb::libusb::from_libusb(e))
                },
            }
        };
    }

    /// Initializes libusb.
    fn init_lib() {
        static INIT: Once = Once::new();
        // SAFETY: C API calls
        INIT.call_once(|| unsafe {
            let v = rusb::version();
            info!(
                "libusb version: {}.{}.{}.{}{}",
                v.major(),
                v.minor(),
                v.micro(),
                v.nano(),
                v.rc().unwrap_or("")
            );
            debug!("- LIBUSB_CAP_HAS_CAPABILITY = {}", rusb::has_capability());
            debug!("- LIBUSB_CAP_HAS_HOTPLUG = {}", rusb::has_hotplug());
            debug!(
                "- LIBUSB_CAP_SUPPORTS_DETACH_KERNEL_DRIVER = {}",
                rusb::supports_detach_kernel_driver()
            );
            libusb_set_log_cb(null_mut(), Some(log_cb), LIBUSB_LOG_CB_GLOBAL);
            let rc = libusb_set_option(null_mut(), LIBUSB_OPTION_LOG_LEVEL, LIBUSB_LOG_LEVEL_DEBUG);
            if rc != LIBUSB_SUCCESS {
                warn!("Failed to enable libusb logging");
            }
        });
    }

    /// Creates a new libusb context.
    pub(super) fn new_ctx(max_log_level: c_int) -> rusb::Result<Context> {
        init_lib();
        let ctx = Context::new()?;
        if cfg!(windows) {
            match check!(libusb_set_option(ctx.as_raw(), LIBUSB_OPTION_USE_USBDK)) {
                Ok(()) => info!("Using UsbDk backend"),
                Err(Error::NotFound) => info!("Using WinUSB backend"),
                Err(e) => return Err(e),
            }
        }
        check!(libusb_set_option(
            ctx.as_raw(),
            LIBUSB_OPTION_LOG_LEVEL,
            max_log_level,
        ))?;
        Ok(ctx)
    }

    /// Resets the specified device handle.
    pub(super) fn reset<T: UsbContext>(hdl: DeviceHandle<T>) -> rusb::Result<DeviceHandle<T>> {
        let dev = hdl.device();
        let port = dev.port_numbers()?;
        // WinUSB API with libusbK driver requires interface 0 to be claimed in
        // order to perform an actual device reset:
        // https://github.com/libusb/libusb/issues/1261
        if let Err(e) = hdl.claim_interface(0) {
            warn!("Failed to claim interface 0 before reset: {e}");
        }
        info!("Resetting {dev:?}");
        let ctx = match hdl.reset() {
            Ok(_) => return Ok(hdl),
            Err(Error::NotFound) => {
                let ctx = hdl.context().clone();
                drop(hdl);
                ctx
            }
            Err(e) => return Err(e),
        };
        info!("Attempting to re-open device");
        let all = ctx.devices()?;
        for dev in all.iter() {
            match dev.port_numbers() {
                Ok(p) if p == port => return dev.open(),
                _ => {}
            }
        }
        error!("Failed to find device after reset");
        Err(Error::NoDevice)
    }

    /// Converts libusb error code to [`Error`]. From `rusb-0.9.4/src/error.rs`
    pub(crate) const fn from_libusb(rc: c_int) -> Error {
        match rc {
            LIBUSB_ERROR_IO => Error::Io,
            LIBUSB_ERROR_INVALID_PARAM => Error::InvalidParam,
            LIBUSB_ERROR_ACCESS => Error::Access,
            LIBUSB_ERROR_NO_DEVICE => Error::NoDevice,
            LIBUSB_ERROR_NOT_FOUND => Error::NotFound,
            LIBUSB_ERROR_BUSY => Error::Busy,
            LIBUSB_ERROR_TIMEOUT => Error::Timeout,
            LIBUSB_ERROR_OVERFLOW => Error::Overflow,
            LIBUSB_ERROR_PIPE => Error::Pipe,
            LIBUSB_ERROR_INTERRUPTED => Error::Interrupted,
            LIBUSB_ERROR_NO_MEM => Error::NoMem,
            LIBUSB_ERROR_NOT_SUPPORTED => Error::NotSupported,
            LIBUSB_ERROR_OTHER | _ => Error::Other,
        }
    }

    extern "system" fn log_cb(_: *mut libusb_context, lvl: c_int, msg: *mut c_void) {
        let r = std::panic::catch_unwind(|| {
            // SAFETY: msg is a valid C string
            let orig = unsafe { CStr::from_ptr(msg as *const c_char) }.to_string_lossy();
            let msg = match orig.as_ref().split_once("libusb: ") {
                Some((_, tail)) => tail.trim_end(),
                _ => return, // Debug header (see log_v() in libusb/core.c)
            };
            match lvl {
                LIBUSB_LOG_LEVEL_ERROR => error!("{}", msg.trim_start_matches("error ")),
                LIBUSB_LOG_LEVEL_WARNING => warn!("{}", msg.trim_start_matches("warning ")),
                LIBUSB_LOG_LEVEL_INFO => debug!("{}", msg.trim_start_matches("info ")),
                LIBUSB_LOG_LEVEL_DEBUG => trace!("{}", msg.trim_start_matches("debug ")),
                _ => trace!("{}", msg.trim_start_matches("unknown ")),
            }
        });
        if let Err(e) = r {
            eprintln!("libusb log callback panic: {e:?}");
            std::process::abort();
        }
    }

}