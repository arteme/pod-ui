use pod_core::controller::Controller;
use pod_core::model::Config;
use pod_gtk::prelude::*;
use anyhow::*;
use log::*;
use crate::config;
use crate::config::XtPacks;

fn is_sensitive(packs: XtPacks, name: &str) -> bool {
    let ms = name.starts_with("MS-");
    let cc = name.starts_with("CC-");
    let bx = name.starts_with("BX-");
    let fx = name.starts_with("FX-");

    (!ms && !cc && !bx && !fx) ||
        (ms && packs.contains(XtPacks::MS)) ||
        (cc && packs.contains(XtPacks::CC)) ||
        (bx && packs.contains(XtPacks::BX)) ||
        (fx && packs.contains(XtPacks::FX))
}

fn init_combo(packs: XtPacks, objs: &ObjectList, name: &str, items: Vec<&str>) -> Result<()> {
    let select = objs.ref_by_name::<gtk::ComboBox>(name)?;

    let list_store = gtk::ListStore::new(
        &[u8::static_type(), String::static_type(), bool::static_type()]
    );

    for (i, item) in items.iter().enumerate() {
        let sensitive = is_sensitive(packs, item);
        list_store.insert_with_values(None, &[
            (0, &(i as u32)), (1, item), (2, &sensitive)
        ]);
    }

    select.set_model(Some(&list_store));
    select.clear();
    select.set_entry_text_column(1);

    let renderer = gtk::CellRendererText::new();
    select.pack_start(&renderer, true);
    select.add_attribute(&renderer, "text", 1);
    select.add_attribute(&renderer, "sensitive", 2);

    Ok(())
}

pub fn init_amp_models(packs: XtPacks, objs: &ObjectList, config: &Config) -> Result<()> {
    let items = config.amp_models.iter().map(|a| a.name.as_str()).collect::<Vec<_>>();
    return init_combo(packs, objs, "amp_select", items);
}
pub fn init_cab_models(packs: XtPacks, objs: &ObjectList, config: &Config) -> Result<()> {
    let items = config.cab_models.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    return init_combo(packs, objs, "cab_select", items);
}

// todo: when switching to BX cab update the mic names from BC_MIC_NAMES!
pub fn init_mic_models(objs: &ObjectList) -> Result<()> {
    let select = objs.ref_by_name::<gtk::ComboBox>("mic_select")?;

    let list_store = gtk::ListStore::new(
        &[u8::static_type(), String::static_type(), bool::static_type()]
    );

    for (i, item) in config::MIC_NAMES.iter().enumerate() {
        list_store.insert_with_values(None, &[
            (0, &(i as u32)), (1, item), (2, &true)
        ]);
    }

    select.set_model(Some(&list_store));
    select.clear();
    select.set_entry_text_column(1);

    let renderer = gtk::CellRendererText::new();
    select.pack_start(&renderer, true);
    select.add_attribute(&renderer, "text", 1);

    Ok(())

}