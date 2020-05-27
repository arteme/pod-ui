use crate::model::Config;

lazy_static! {
    pub static ref PODS: Vec<Config> = vec![
        Config {
            name: "POD 2.0".to_string(),
            family: 0x0000,
            member: 0x0300,
            amp_models: vec![],
            cab_models: vec![],
            controls: Default::default()
        }
    ];
}

