#[derive(Clone, Debug)]
pub struct DeviceAddedEvent {
    pub vid: u16,
    pub pid: u16
}

#[derive(Clone, Debug)]
pub struct DeviceRemovedEvent {
    pub vid: u16,
    pub pid: u16
}

#[derive(Clone, Debug)]
pub enum UsbEvent {
    DeviceAdded(DeviceAddedEvent),
    DeviceRemoved(DeviceRemovedEvent),
    InitDone
}