use ripple_sdk::{
    export_ripple_plugin,
    plugin::{RippleExtension, RipplePlugin, ExtensionLibrary}, export_extn_library,
};

pub mod governance {
    pub mod xvp_privacy;
}

#[derive(Debug, Default)]
pub struct XVPPlugin;
impl RipplePlugin for XVPPlugin {
    fn get_extensions(&self) -> Vec<RippleExtension> {
        vec![
            RippleExtension::DataGovernanceChannel(Box::new(
                crate::governance::xvp_privacy::XVPPrivacyChannel::default(),
            )),
            RippleExtension::JsonRpseeExtension(Box::new(
                crate::governance::xvp_privacy::XvpPrivacyExtn,
            )),
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

fn init_library() -> ExtensionLibrary {
    ExtensionLibrary {
        name: "xvp".into(),
        extensions: vec![
            RippleExtension::DataGovernanceChannel(Box::new(
                crate::governance::xvp_privacy::XVPPrivacyChannel::default(),
            )),
            RippleExtension::JsonRpseeExtension(Box::new(
                crate::governance::xvp_privacy::XvpPrivacyExtn,
            ))
        ]
    }
}
export_extn_library!(ExtensionLibrary, init_library);

fn init() -> Box<dyn RipplePlugin> {
    Box::new(XVPPlugin {})
}

export_ripple_plugin!(XVPPlugin, XVPPlugin::default);
