use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use std::cell::RefCell;

    use gtk::glib::Properties;

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ModObject)]
    pub struct ModObject {
        // https://github.com/gtk-rs/gtk-rs-core/issues/930
        #[property(get, set)]
        pub enabled: RefCell<bool>,
        #[property(get, set)]
        pub name: RefCell<String>,
        #[property(get, set)]
        pub is_local: RefCell<bool>,
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
    pub fn new(enabled: bool, name: String, is_local: bool) -> Self {
        glib::Object::builder()
            .property("enabled", enabled)
            .property("name", name)
            .property("is_local", is_local)
            .build()
    }
}

impl Default for ModObject {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
