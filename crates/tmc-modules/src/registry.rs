use crate::combat::Combat;
use crate::dashboard::Dashboard;
use crate::exobiology::Exobiology;
use crate::exploration::Exploration;
use crate::mining::Mining;
use crate::navigation::Navigation;
use tmc_core::Module;

pub fn all_modules() -> Vec<Box<dyn Module>> {
    vec![
        Box::new(Dashboard::new()),
        Box::new(Navigation::new()),
        Box::new(Exploration::new()),
        Box::new(Exobiology::new()),
        Box::new(Combat::new()),
        Box::new(Mining::new()),
    ]
}
