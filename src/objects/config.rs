use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use noitad_lib::{config, impl_deref_for_newtype, noita};

#[derive(Debug, Clone, Default, glib::Boxed)]
#[boxed_type(name = "NoitadModProfilesBoxed")]
pub struct ModProfiles(pub noita::ModProfiles);
impl_deref_for_newtype!(ModProfiles, noita::ModProfiles);

#[derive(Debug, Clone, Default, glib::Boxed)]
#[boxed_type(name = "NoitadNoitaPathBoxed")]
pub struct NoitaPath(pub noita::NoitaPath);
impl_deref_for_newtype!(NoitaPath, noita::NoitaPath);

mod imp {
    use std::cell::RefCell;

    use gtk::glib::Properties;

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ConfigObject)]
    pub struct ConfigObject {
        #[property(get, set)]
        pub noita_path: RefCell<NoitaPath>,
        #[property(get, set)]
        pub profiles: RefCell<ModProfiles>,
        #[property(get, set)]
        pub active_profile: RefCell<Option<String>>,
        #[property(get, set)]
        pub active_profile_sync: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConfigObject {
        const NAME: &'static str = "NoitadConfigObject";
        type Type = super::ConfigObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ConfigObject {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
}

glib::wrapper! {
    pub struct ConfigObject(ObjectSubclass<imp::ConfigObject>);
}

impl ConfigObject {
    pub fn new(config: config::Config) -> Self {
        glib::Object::builder()
            .property("noita_path", NoitaPath(config.noita_path))
            .property("profiles", ModProfiles(config.profiles))
            .property("active_profile", config.active_profile)
            .property("active_profile_sync", config.active_profile_sync)
            .build()
    }
}

impl Default for ConfigObject {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl Into<config::Config> for ConfigObject {
    fn into(self) -> config::Config {
        config::Config {
            noita_path: self.noita_path().to_owned().0,
            profiles: self.profiles().to_owned().0,
            active_profile: self.active_profile().to_owned(),
            active_profile_sync: self.active_profile_sync().to_owned(),
        }
    }
}
