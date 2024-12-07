use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib, NoSelection, SignalListItemFactory, StringList};
use itertools::Itertools;
use noitad_lib::config::Config;
use noitad_lib::defines::APP_CONFIG_PATH;

use crate::application::NoitadApplication;
use crate::config::{APP_ID, PROFILE};
use crate::models::mod_::ModObject;
use crate::widgets::mod_entry_row::ModEntryRow;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nozwock/noitad/ui/window.ui")]
    pub struct NoitadApplicationWindow {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub dropdown_profile: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub mod_list: TemplateChild<gtk::ListView>,
        pub settings: gio::Settings,
        pub config: RefCell<Option<Config>>,
    }

    impl Default for NoitadApplicationWindow {
        fn default() -> Self {
            Self {
                stack: Default::default(),
                dropdown_profile: Default::default(),
                mod_list: Default::default(),
                settings: gio::Settings::new(APP_ID),
                config: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NoitadApplicationWindow {
        const NAME: &'static str = "NoitadApplicationWindow";
        type Type = super::NoitadApplicationWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            ModEntryRow::ensure_type();

            klass.bind_template();
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NoitadApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Devel Profile
            if PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            if let Ok(cfg) = Config::load() {
                self.config.replace(Some(cfg));
            }

            // Load latest window state
            obj.load_window_size();
            obj.setup_ui();
        }
    }

    impl WidgetImpl for NoitadApplicationWindow {}
    impl WindowImpl for NoitadApplicationWindow {
        // Save window state on delete event
        fn close_request(&self) -> glib::Propagation {
            if let Err(err) = self.obj().save_window_size() {
                tracing::warn!("Failed to save window state, {}", &err);
            }

            _ = self.obj().get_config();

            // Pass close request on to the parent
            self.parent_close_request()
        }
    }

    impl ApplicationWindowImpl for NoitadApplicationWindow {}
    impl AdwApplicationWindowImpl for NoitadApplicationWindow {}
}

glib::wrapper! {
    pub struct NoitadApplicationWindow(ObjectSubclass<imp::NoitadApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

impl NoitadApplicationWindow {
    pub fn new(app: &NoitadApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        let (width, height) = self.default_size();

        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;

        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let imp = self.imp();

        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");
        let is_maximized = imp.settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // dbg!(APP_CONFIG_PATH.as_path());

        let stack = imp.stack.get();
        let dropdown_profile = imp.dropdown_profile.get();

        // Temporary for testing
        stack.set_visible_child_name("main_page");

        let cfg = self.get_config();

        let profiles = cfg
            .profiles
            .keys()
            .into_iter()
            .map(|it| it.as_str())
            .collect_vec();
        dbg!(&profiles);

        let string_list = StringList::new(&profiles);
        dropdown_profile.set_model(Some(&string_list));

        let mod_list = imp.mod_list.get();

        let model = gio::ListStore::new::<ModObject>();
        let mods = cfg
            .profiles
            .get_profile(cfg.active_profile.as_ref().unwrap())
            .unwrap();
        let mod_objs = mods
            .mods
            .iter()
            .map(|it| ModObject::new(it.enabled, it.name.clone(), it.workshop_item_id == 0))
            .collect_vec();
        model.extend_from_slice(&mod_objs);

        let selection_model = NoSelection::new(Some(model));
        mod_list.set_model(Some(&selection_model));

        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let mod_row = ModEntryRow::new();
            list_item
                .downcast_ref::<gtk::ListItem>()
                .unwrap()
                .set_child(Some(&mod_row));
        });
        factory.connect_bind(move |_, list_item| {
            let mod_object = list_item
                .downcast_ref::<gtk::ListItem>()
                .unwrap()
                .item()
                .and_downcast::<ModObject>()
                .unwrap();

            let mod_row = list_item
                .downcast_ref::<gtk::ListItem>()
                .unwrap()
                .child()
                .and_downcast::<ModEntryRow>()
                .unwrap();

            mod_row.bind(&mod_object);
        });
        factory.connect_unbind(move |_, list_item| {
            let mod_row = list_item
                .downcast_ref::<gtk::ListItem>()
                .unwrap()
                .child()
                .and_downcast::<ModEntryRow>()
                .unwrap();

            mod_row.unbind();
        });
        mod_list.set_factory(Some(&factory));
    }

    fn get_config(&self) -> Config {
        self.imp().config.borrow().clone().unwrap()
    }
}
