use crate::dashboard::Dashboard;
use tmc_core::Module;

pub fn all_modules() -> Vec<Box<dyn Module>> {
    vec![Box::new(Dashboard::new())]
}
