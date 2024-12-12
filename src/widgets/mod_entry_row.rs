use adw::subclass::prelude::*;
use gtk::{glib, prelude::ObjectExt};

use crate::objects::noita_mod::ModObject;

mod imp {
    use std::cell::RefCell;

    use glib::Binding;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nozwock/noitad/ui/mod_entry_row.ui")]
    pub struct ModEntryRow {
        #[template_child]
        pub enabled: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub mod_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub is_local: TemplateChild<gtk::Label>,
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ModEntryRow {
        const NAME: &'static str = "NoitadModEntryRow";
        type Type = super::ModEntryRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // #[glib::derived_properties]
    impl ObjectImpl for ModEntryRow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for ModEntryRow {}
    impl BoxImpl for ModEntryRow {}
    impl ListBoxRowImpl for ModEntryRow {}
    impl PreferencesRowImpl for ModEntryRow {}
    impl ActionRowImpl for ModEntryRow {}
}

glib::wrapper! {
    pub struct ModEntryRow(ObjectSubclass<imp::ModEntryRow>)
        @extends gtk::Widget, gtk::Box, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ModEntryRow {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn bind(&self, mod_object: &ModObject) {
        let enabled = self.imp().enabled.get();
        let mod_name = self.imp().mod_name.get();
        let is_local = self.imp().is_local.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let enabled_binding = mod_object
            .bind_property("enabled", &enabled, "active")
            .bidirectional()
            .sync_create()
            .build();
        bindings.push(enabled_binding);

        let content_label_binding = mod_object
            .bind_property("name", &mod_name, "label")
            .sync_create()
            .build();
        bindings.push(content_label_binding);

        let content_label_binding = mod_object
            .bind_property("is_local", &is_local, "label")
            .sync_create()
            .transform_to(|_, active| Some(if active { "Local" } else { "Steam" }))
            .build();
        bindings.push(content_label_binding);
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}

impl Default for ModEntryRow {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
