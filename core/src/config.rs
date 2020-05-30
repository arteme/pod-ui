use crate::model::Config;
use once_cell::sync::Lazy;
use crate::model::*;


macro_rules! def {
    () => (::std::default::Default::default(););
}

pub static _PODS: Lazy<Vec<Config>> = Lazy::new(|| {
   vec![
       Config {
           name: "POD 2.0".to_string(),
           family: 0x0000,
           member: 0x0300,

           program_size: 71,
           all_programs_size: 71 * 36,
           pod_id: 0x01,

           amp_models: vec![],
           cab_models: vec![],
           controls: convert_args!(hashmap!(
               // 12: amp model
               "drive" => RangeControl { cc: 13, ..def!() },
               // drive 2 (no transmit)
               "bass" => RangeControl { cc: 14, ..def!() },
               "mid" => RangeControl { cc: 15, ..def!() },
               "treble" => RangeControl { cc: 16, ..def!() },
               "bright switch" => RangeControl { cc: 73, ..def!() }
               // 21: presence | tx on/off | rx 0-127


           ))
       }
   ]
});

// lazy_static! confuses IDEA type inference, making using _PODS directly in code hard
#[allow(non_snake_case)]
pub fn PODS() -> &'static Vec<Config> {
    &_PODS
}
