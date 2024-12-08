use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib::clone;
use gtk::{gio, glib, NoSelection, SignalListItemFactory, StringList};
use itertools::Itertools;
use noitad_lib::config::Config;
use noitad_lib::defines::APP_CONFIG_PATH;
use tracing::error;

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
        pub config: Rc<RefCell<Config>>,
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
                self.config.replace(cfg);
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

            _ = dbg!(dbg!(self.config.borrow()).store());

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

        // note: Temporary for testing
        stack.set_visible_child_name("main_page");

        let cfg = imp.config.clone();
        let cfg_ref = cfg.borrow();

        let profiles = cfg_ref
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
        let mods = cfg_ref
            .profiles
            .get_profile(cfg_ref.active_profile.as_ref().unwrap())
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

    pub fn profile_new(&self) {
        fn dialog_profile() -> (adw::AlertDialog, adw::EntryRow) {
            let dialog = adw::AlertDialog::builder()
                .close_response("cancel")
                .heading("New Profile")
                .build();

            dialog.add_responses(&[("cancel", "Cancel"), ("create", "Create")]);
            dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
            dialog.set_response_enabled("create", false);

            let box_ = gtk::Box::builder()
                .margin_top(12)
                .spacing(24)
                .orientation(gtk::Orientation::Vertical)
                .build();
            let list_box = gtk::ListBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .build();
            list_box.add_css_class("boxed-list");
            let entry_row = adw::EntryRow::builder().title("Profile Name").build();

            entry_row.connect_text_notify(clone!(
                #[weak]
                dialog,
                move |entry| {
                    let text = entry.text();
                    if !text.is_empty() {
                        dialog.set_response_enabled("create", true);
                    } else {
                        dialog.set_response_enabled("create", false);
                    }
                }
            ));

            list_box.append(&entry_row);
            box_.append(&list_box);
            dialog.set_extra_child(Some(&box_));

            dialog.set_focus(Some(&entry_row));

            (dialog, entry_row)
        }

        let (dialog, entry_row) = dialog_profile();
        let cfg = self.imp().config.clone();
        let save_dir = cfg.borrow().noita_path.save_dir().unwrap();
        dialog.choose(self, None::<&gio::Cancellable>, move |resp| {
            let text = entry_row.text();
            if resp.as_str() == "create" && !text.is_empty() {
                _ = cfg
                    .as_ref()
                    .borrow_mut()
                    .profiles
                    .add_profile(text, save_dir)
                    .inspect_err(|e| error!(%e));
            }
        });

        // todo: Toast for failure/success
    }
}
