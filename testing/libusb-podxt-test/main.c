/* A test app using ral libusb to connect to a PODxt device (or others), send requests and
 * receive replies, printing sent and received data.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <libusb-1.0/libusb.h>
#include <unistd.h>
#include <pthread.h>

//#define PODXT
#ifdef PODXT
// PODxt
#  define VID 0x0e41
#  define PID 0x5044
#  define CFG 1
#  define IFACE 0
#  define ALT 5
#  define READ_EP 0x84
#  define WRITE_EP 0x03

//#  define REQ { 0xf0, 0x7e, 0x7f, 0x06, 0x01, 0xf7 } // UDI
#  define REQ { 0xf0, 0x00, 0x01, 0x0c, 0x03, 0x75, 0x7f }
#else
// PocketPOD
#  define VID 0x0e41
#  define PID 0x5051
#  define CFG 1
#  define IFACE 1
#  define ALT 0
#  define READ_EP 0x82
#  define WRITE_EP 0x02

#  define REQ { 0x04, 0xf0, 0x7e, 0x7f, 0x07, 0x06, 0x01, 0xf7 } // UDI, USB-MIDI framing
#endif

static struct libusb_context *ctx = NULL;
static struct libusb_device_handle* dev_handle = NULL;

#define ERR(...) fprintf(stderr, __VA_ARGS__)

void my_exit(int error_code) {
    if (dev_handle) libusb_close(dev_handle);
    if (ctx) libusb_exit(ctx);
    exit(error_code);
}
#define exit my_exit

void print_buffer(unsigned char* buffer, unsigned int len) {
    printf("[");
    for(int i = 0; i < len; i++) {
        printf("%02x%s", buffer[i], i < len - 1 ? " " : "");
    }
    printf("] len=%d", len);
}

void usb_send_cb(struct libusb_transfer* t) {
    printf(">> ");
    print_buffer(t->buffer, t->actual_length);
    if (t->length != t->actual_length) printf("(%d) ", t->length);
    if (t->status != LIBUSB_TRANSFER_COMPLETED) printf("status=%d", t->status);
    printf("\n");
}

void usb_send(unsigned char* bytes, unsigned int len) {
    printf("usb_send %p\n", dev_handle);
    struct libusb_transfer* t = libusb_alloc_transfer(0);
    libusb_fill_bulk_transfer(t, dev_handle, WRITE_EP, bytes, len, usb_send_cb, NULL, 0);
    //libusb_fill_interrupt_transfer(t, dev_handle, WRITE_EP, bytes, len, usb_send_cb, NULL, 0);
    t->flags = LIBUSB_TRANSFER_FREE_TRANSFER;

    int res = libusb_submit_transfer(t);
    if (res != LIBUSB_SUCCESS) {
        printf("usb_send: libusb_submit_transfer error: %d\n", res);
    }
}

void usb_recv_cb(struct libusb_transfer* t) {
    printf("<< ");
    print_buffer(t->buffer, t->actual_length);
    if (t->length != t->actual_length) printf("(%d) ", t->length);
    if (t->status != LIBUSB_TRANSFER_COMPLETED) printf("status=%d", t->status);
    printf("\n");

    // resubmit
    int res = libusb_submit_transfer(t);
    if (res != LIBUSB_SUCCESS) {
        printf("usb_recv_cb: libusb_submit_transfer error: %d\n", res);
    }
}

void usb_recv(unsigned char* bytes, unsigned int len) {
    printf("recv %p\n", dev_handle);
    struct libusb_transfer* t = libusb_alloc_transfer(0);
    libusb_fill_bulk_transfer(t, dev_handle, READ_EP, bytes, 16, usb_recv_cb, NULL, 10000);
    //libusb_fill_interrupt_transfer(t, dev_handle, READ_EP, bytes, len, usb_send_cb, NULL, 10000);

    int res = libusb_submit_transfer(t);
    if (res != LIBUSB_SUCCESS) {
        printf("usb_recv: libusb_submit_transfer error: %d\n", res);
    }
}

void* sender(void *data) {
    while(1) {
        unsigned char buffer1[] = REQ;
        usb_send(buffer1, sizeof(buffer1));

        sleep(5);
    }
}

void print_interface(const struct libusb_interface* iface) {
    for(int i = 0; i < iface->num_altsetting; i++) {
        const struct libusb_interface_descriptor* d = &iface->altsetting[i];
        char alt_buffer[32] = { 0 };
        if (d->bAlternateSetting != 0) {
            snprintf(alt_buffer, 32, "/%d", d->bAlternateSetting);
        }

        printf("  Interface %d%s:\n", d->bInterfaceNumber, alt_buffer);
        printf("    interface number : %d\n", d->bInterfaceNumber);
        printf("    alt setting      : %d\n", d->bAlternateSetting);
        printf("    class            : %d\n", d->bInterfaceClass);
        printf("    sub-class        : %d\n", d->bInterfaceSubClass);
        printf("    endpoints number : %d\n", d->bNumEndpoints);
        printf("\n");
        for (int j = 0; j < d->bNumEndpoints; j++) {
            const struct libusb_endpoint_descriptor* e = &d->endpoint[j];
            printf("    Endpoint:\n");
            printf("      address         : %02x\n", e->bEndpointAddress);
            printf("      max packet size : %u\n", e->wMaxPacketSize);
        }
    }
}

void print_config(struct libusb_config_descriptor* cfg) {
    printf("Configuration:\n");
    printf("   value             : %d\n", cfg->bConfigurationValue);
    printf("   interfaces number : %d\n", cfg->bNumInterfaces);
    printf("\n");
    for (int i = 0; i < cfg->bNumInterfaces; i++) {
        print_interface(&cfg->interface[i]);
    }
}

void print_device(libusb_device_handle* h) {
    libusb_device* d = libusb_get_device(h);
    struct libusb_device_descriptor desc;

    int res = libusb_get_device_descriptor(d, &desc);
    if (res < 0) {
        ERR("Failed to get device descriptor\n");
        return;
    }

    printf("Bus %u, device %u: %04x:%04x\n",
           libusb_get_bus_number(d), libusb_get_device_address(d), desc.idVendor, desc.idProduct);
    printf("Configurations number: %d\n", desc.bNumConfigurations);

    for(int i = 0; i < desc.bNumConfigurations; i++) {
        struct libusb_config_descriptor* cfg;
        res = libusb_get_config_descriptor(d, i, &cfg);
        if (res != LIBUSB_SUCCESS) {
            ERR("Failed to get config descriptor %d\n", i);
            continue;
        }
        print_config(cfg);
        libusb_free_config_descriptor(cfg);
    }
}

int main() {
    const struct libusb_version* ver = libusb_get_version();
    printf("libusb: %d.%d.%d.%d%s\n", ver->major, ver->minor, ver->micro, ver->nano, ver->rc);

    int r = libusb_init_context(&ctx, NULL, 0);
    if (r < 0) {
        ERR("Error initializing libusb: %s\n", libusb_error_name(r));
        exit(1);
    }

    dev_handle = libusb_open_device_with_vid_pid(ctx, VID, PID);
    if (!dev_handle) {
        ERR("Failed to find device %x:%x\n", VID, PID);
        exit(1);
    }
    print_device(dev_handle);


    sleep(2);


    //libusb_reset_device(dev_handle);


    //libusb_set_auto_detach_kernel_driver(dev_handle, 1);
    libusb_detach_kernel_driver(dev_handle, 0);
    libusb_detach_kernel_driver(dev_handle, IFACE);

    libusb_reset_device(dev_handle);

    r = libusb_set_configuration(dev_handle, CFG);
    if (r != LIBUSB_SUCCESS) {
        ERR("set_config failed: %s\n", libusb_error_name(r));
    }

    r = libusb_claim_interface(dev_handle, 0);
    if (r != LIBUSB_SUCCESS) {
        ERR("claim_interface failed: %s\n", libusb_error_name(r));
    }
    r = libusb_claim_interface(dev_handle, IFACE);
    if (r != LIBUSB_SUCCESS) {
        ERR("claim_interface failed: %s\n", libusb_error_name(r));
    }
    /*
    r = libusb_set_interface_alt_setting(dev_handle, 0, ALT);
    if (r != LIBUSB_SUCCESS) {
        ERR("set_interface_alt_setting failed: %s\n", libusb_error_name(r));
    }
    r = libusb_set_interface_alt_setting(dev_handle, 1, ALT);
    if (r != LIBUSB_SUCCESS) {
        ERR("set_interface_alt_setting failed: %s\n", libusb_error_name(r));
    }
     */

    // start readers
    for (int i = 0; i < 1; i++) {
        unsigned char* buffer = malloc(1024);
        usb_recv(buffer, 1024);
    }

    // start writer
    pthread_t thread;
    pthread_create(&thread, NULL, &sender, NULL);

    ERR("starting event loop\n");
    while(1) {
        int r = libusb_handle_events(ctx);
        if (r != LIBUSB_SUCCESS) {
            ERR("Event loop failed: %s\n", libusb_error_name(r));
            break;
        }
        printf("tick\n");
    }

    exit(0);
}
