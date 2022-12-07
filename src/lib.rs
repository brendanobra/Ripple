use ripple_sdk::{plugin::{RipplePlugin, RippleExtension}, export_ripple_plugin};

pub mod governance {
    pub mod privacy;
}

struct XVPPlugin;
impl RipplePlugin for XVPPlugin {

    fn get_extensions(&self) -> Vec<RippleExtension> {
        vec![
            RippleExtension::DataGovernanceChannel(Box::new(crate::governance::privacy::XVPPrivacyChannel)),
            RippleExtension::JsonRpseeExtension(Box::new(crate::governance::privacy::XvpPrivacyExtn))
            ]
    }

    fn name(&self) -> &'static str {
        "xvp"
    }

    fn on_load(&self) {
        println!("Loading xvp plugin")
    }

    fn on_unload(&self) {
        println!("Unloading xvp plugin")
    }
}

fn init() -> Box<dyn RipplePlugin> {
    Box::new(XVPPlugin {})
}

export_ripple_plugin!(init);