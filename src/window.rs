use std::borrow::Borrow;
use std::cell::RefMut;
use std::collections::HashMap;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gio::ListStore;
use gtk::glib::clone;
use gtk::{gio, glib, StringObject};
use itertools::Itertools;
use noitad_lib::config::Config;
use tracing::{debug, error};

use crate::application::NoitadApplication;
use crate::config::{APP_ID, PROFILE};
use crate::objects::config::ModProfiles;
use crate::objects::noita_mod::ModObject;

mod imp {
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    use crate::objects::config::ConfigObject;

    use super::*;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nozwock/noitad/ui/window.ui")]
    pub struct NoitadApplicationWindow {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub dropdown_profile: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub mod_list: TemplateChild<gtk::ListBox>,
        pub settings: gio::Settings,
        pub config: ConfigObject,

        pub mod_list_models: Rc<RefCell<HashMap<String, Vec<ModObject>>>>,
    }

    impl Default for NoitadApplicationWindow {
        fn default() -> Self {
            Self {
                stack: Default::default(),
                dropdown_profile: Default::default(),
                mod_list: Default::default(),
                settings: gio::Settings::new(APP_ID),
                config: Default::default(),
                mod_list_models: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NoitadApplicationWindow {
        const NAME: &'static str = "NoitadApplicationWindow";
        type Type = super::NoitadApplicationWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
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
                self.config.set_config(cfg);
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

            _ = dbg!(dbg!(self.config.into_simple_config()).store());

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

        let stack = imp.stack.get();
        let dropdown_profile = imp.dropdown_profile.get();
        let mod_list = imp.mod_list.get();
        let mod_list_models = imp.mod_list_models.clone();

        // note: Temporary for testing
        stack.set_visible_child_name("main_page");

        let cfg = &imp.config;

        fn sync_profiles_model(profiles: &ModProfiles, model: &ListStore) {
            model.retain(|s| {
                profiles.contains_key(s.downcast_ref::<StringObject>().unwrap().string().as_str())
            });

            // Need to keep a list of these StringObjects stored in the model,
            // since this `model.find` seems to work on object id rather than value
            let mut item_store = vec![];
            {
                let mut i = 0;
                while let Some(obj) = model.item(i) {
                    item_store.push(obj.downcast::<StringObject>().unwrap());
                    i = i + 1;
                }

                profiles.keys().for_each(|s| {
                    if item_store
                        .iter()
                        .find(|obj| obj.string().as_str() == s)
                        .is_none()
                    {
                        item_store.push(StringObject::new(s));
                    }
                });
            }

            // It's important that items are sorted in the same manner throughout all these collections...
            item_store
                .iter()
                .sorted_by(|a, b| Ord::cmp(a.string().as_str(), b.string().as_str()))
                .enumerate()
                .for_each(|(i, s)| {
                    if model.find(s).is_none() {
                        model.splice(i as u32, 0, &[s.clone()]);
                    }
                });
        }

        let profiles_model = ListStore::new::<StringObject>();
        sync_profiles_model(&cfg.profiles(), &profiles_model);
        cfg.connect_profiles_notify(clone!(
            #[weak]
            profiles_model,
            move |cfg| {
                let profiles = cfg.profiles();
                sync_profiles_model(&profiles, &profiles_model);
            }
        ));
        dropdown_profile.set_model(Some(&profiles_model));

        let mod_list_model = gio::ListStore::new::<ModObject>();

        if let Some(active_profile) = cfg.active_profile() {
            dropdown_profile.set_selected(
                cfg.profiles()
                    .keys()
                    .sorted()
                    .enumerate()
                    .find_map(|(i, s)| {
                        if s.as_str() == active_profile {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .unwrap() as u32,
            );
        }
        dropdown_profile.connect_selected_item_notify(clone!(
            #[weak]
            cfg,
            #[weak]
            mod_list_model,
            #[weak]
            mod_list_models,
            move |dropdown| {
                let str_obj = dropdown.selected_item().unwrap();
                let active_profile = str_obj.downcast_ref::<StringObject>().unwrap().string();
                cfg.set_active_profile(active_profile.as_str());

                // Update mod_list
                let is_model_cached = mod_list_models
                    .as_ref()
                    .borrow()
                    .contains_key(active_profile.as_str());

                debug!(
                    name = active_profile.as_str(),
                    is_model_cached, "Selected profile"
                );

                let mod_objs = if is_model_cached {
                    mod_list_models
                        .as_ref()
                        .borrow()
                        .get(active_profile.as_str())
                        .unwrap()
                        .into_iter()
                        .map(|it| it.clone())
                        .collect_vec()
                } else {
                    let mod_objs = profile_mod_objs(
                        &cfg.profiles(),
                        &active_profile,
                        mod_list_models.as_ref().borrow_mut(),
                    );

                    mod_objs
                };

                mod_list_model.remove_all();
                mod_list_model.extend_from_slice(&mod_objs);
            }
        ));

        fn profile_mod_objs(
            profiles: &ModProfiles,
            active: impl AsRef<str>,
            mut mods_store: RefMut<HashMap<String, Vec<ModObject>>>,
        ) -> Vec<ModObject> {
            let mods = profiles.get_profile(active.as_ref()).unwrap();
            let mod_objs = mods
                .mods
                .iter()
                .map(|it| ModObject::new(it.enabled, it.name.clone(), it.workshop_item_id == 0))
                .collect_vec();
            mods_store.insert(active.as_ref().to_owned(), mod_objs.clone());

            mod_objs
        }

        // todo: This also needs to be when the first profile is created
        if let Some(active_profile) = cfg.active_profile() {
            let mod_objs = profile_mod_objs(
                &cfg.profiles(),
                &active_profile,
                mod_list_models.as_ref().borrow_mut(),
            );
            mod_list_model.extend_from_slice(&mod_objs);
        }

        mod_list.bind_model(Some(&mod_list_model), move |obj| {
            let item = obj.downcast_ref::<ModObject>().unwrap();
            let row = adw::SwitchRow::builder().title(item.name()).build();

            item.bind_property("enabled", &row, "active")
                .bidirectional()
                .sync_create()
                .build();

            row.into()
        });
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

            (dialog, entry_row)
        }

        let (dialog, entry_row) = dialog_profile();
        let cfg = self.imp().config.clone();
        dialog.choose(self, None::<&gio::Cancellable>, move |resp| {
            let text = entry_row.text();
            if resp.as_str() == "create" && !text.is_empty() {
                let save_dir = cfg.noita_path().save_dir().unwrap();
                let mut profiles = (&cfg).profiles();
                _ = profiles
                    .add_profile(text, save_dir)
                    .inspect_err(|e| error!(%e));
                cfg.set_profiles(profiles);
            }
        });

        // todo: Toast for failure/success
    }
}
