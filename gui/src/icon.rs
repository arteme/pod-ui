use anyhow::*;
use pod_gtk::prelude::*;
use gtk::gdk_pixbuf::*;

#[cfg(not(target_os = "macos"))]
pub fn set_app_icon(window: &gtk::Window) -> Result<()> {
    let bytes = glib::Bytes::from_static(include_bytes!("../resources/icon.gresource"));
    let res = gio::Resource::from_data(&bytes)?;
    gio::resources_register(&res);

    let mut icon_list = vec![];
    for p in gio::resources_enumerate_children("/icon", gio::ResourceLookupFlags::all())?.iter() {
        let pixbuf = Pixbuf::from_resource(&format!("/icon/{}", p))?;
        icon_list.push(pixbuf);
    }
    window.set_icon_list(&icon_list);

    gio::resources_unregister(&res);
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn set_app_icon(_window: &gtk::Window) -> Result<()> {
    Ok(())
}
