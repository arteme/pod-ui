use crate::model::Config;

lazy_static! {
    pub static ref _PODS: Vec<Config> = vec![
        Config {
            name: "POD 2.0".to_string(),
            family: 0x0000,
            member: 0x0300,

            program_size: 71,
            all_programs_size: 71 * 36,
            pod_id: 0x01,

            amp_models: vec![],
            cab_models: vec![],
            controls: Default::default()
        }
    ];
}
// lazy_static! confuses IDEA type inference, making using _PODS directly in code hard
#[allow(non_snake_case)]
pub fn PODS() -> &'static Vec<Config> {
    &_PODS
}
