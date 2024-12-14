use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use noitad_lib::{impl_deref_for_newtype, noita};

#[derive(Debug, Clone, Default, glib::Boxed)]
#[boxed_type(name = "NoitadModsBoxed")]
pub struct Mod(pub noita::mod_config::Mod);
impl_deref_for_newtype!(Mod, noita::mod_config::Mod);

mod imp {
    use std::cell::RefCell;

    use gtk::glib::Properties;

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ModObject)]
    pub struct ModObject {
        #[property(get, set, name = "enabled", type = bool, member = enabled)]
        #[property(get, set, name = "name", type = String, member = name)]
        pub inner: RefCell<Mod>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ModObject {
        const NAME: &'static str = "NoitadModObject";
        type Type = super::ModObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ModObject {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
}

glib::wrapper! {
    pub struct ModObject(ObjectSubclass<imp::ModObject>);
}

impl ModObject {
    pub fn new(mod_: noita::mod_config::Mod) -> Self {
        let obj: Self = glib::Object::builder().build();
        *obj.imp().inner.borrow_mut() = Mod(mod_);

        obj
    }

    pub fn is_local(&self) -> bool {
        self.imp().inner.borrow().is_local()
    }
}

impl Default for ModObject {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
