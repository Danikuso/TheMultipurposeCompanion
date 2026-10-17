use crate::dashboard::Dashboard;
use crate::navigation::Navigation;
use tmc_core::Module;

pub fn all_modules() -> Vec<Box<dyn Module>> {
    vec![Box::new(Dashboard::new()), Box::new(Navigation::new())]
}
