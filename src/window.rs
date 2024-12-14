use std::borrow::BorrowMut;
use std::cell::RefMut;
use std::collections::HashMap;

use adw::prelude::*;
use adw::subclass::prelude::*;
use color_eyre::eyre::Result;
use gtk::gio::ListStore;
use gtk::glib::clone;
use gtk::{gio, glib, StringObject};
use itertools::Itertools;
use noitad_lib::config::Config;
use noitad_lib::noita::mod_config::Mods;
use tracing::{debug, error, info};

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
        #[template_child]
        pub button_save_mod_list: TemplateChild<gtk::Button>,
        pub settings: gio::Settings,
        pub config: ConfigObject,

        pub mod_list_models: Rc<RefCell<HashMap<String, Vec<ModObject>>>>,
        pub is_profile_modified: Rc<RefCell<HashMap<String, bool>>>,
    }

    impl Default for NoitadApplicationWindow {
        fn default() -> Self {
            Self {
                stack: Default::default(),
                dropdown_profile: Default::default(),
                mod_list: Default::default(),
                button_save_mod_list: Default::default(),
                settings: gio::Settings::new(APP_ID),
                config: Default::default(),
                mod_list_models: Default::default(),
                is_profile_modified: Default::default(),
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
            obj.setup_gactions();
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

    fn setup_gactions(&self) {
        let action_profile_new = gio::ActionEntry::builder("profile-new")
            .activate(|window: &Self, _, _| {
                window.present_profile_new_dialog();
            })
            .build();

        self.add_action_entries([action_profile_new]);
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // note: Temporary for testing
        let stack = imp.stack.get();
        stack.set_visible_child_name("main_page");

        let mod_list_model = gio::ListStore::new::<ModObject>();
        self.setup_profile_dropdown(&mod_list_model);
        self.setup_mod_list(&mod_list_model);

        let button_save_mod_list = imp.button_save_mod_list.get();
        button_save_mod_list.connect_clicked(clone!(
            #[weak]
            imp,
            move |btn| {
                btn.set_sensitive(false);

                let is_profile_modified = imp.is_profile_modified.clone();
                let mod_list_models = imp.mod_list_models.clone();
                let mod_list_models_ref = mod_list_models.borrow();
                let mut profiles = imp.config.profiles();
                is_profile_modified
                    .borrow()
                    .iter()
                    .filter_map(
                        |(profile, is_modified)| {
                            if *is_modified {
                                Some(profile)
                            } else {
                                None
                            }
                        },
                    )
                    .map(|profile| {
                        (
                            profile,
                            mod_objs_to_mods(mod_list_models_ref.get(profile).unwrap()),
                        )
                    })
                    .for_each(|(profile, mods)| {
                        info!(%profile, "Serializing");
                        _ = profiles
                            .borrow_mut()
                            .update_profile(profile, &mods)
                            .inspect_err(|e| error!(%e));
                    });

                btn.set_visible(false);
                btn.set_sensitive(true);
            }
        ));
    }

    fn setup_profile_dropdown(&self, mod_list_model: &ListStore) {
        let imp = self.imp();
        let cfg = &imp.config;

        let dropdown_profile = imp.dropdown_profile.get();
        let mod_list_models = imp.mod_list_models.clone();

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
                    let mod_objs = Self::get_profile_mod_objs(
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
    }

    fn setup_mod_list(&self, mod_list_model: &ListStore) {
        let imp = self.imp();
        let cfg = imp.config.clone();

        let mod_list = imp.mod_list.get();
        let mod_list_models = imp.mod_list_models.clone();

        // todo: Aside from the sync for the active profile with noita mod_config,
        // there needs to be sync for others where new mod entries are added in those profiles
        // when they're being loaded

        // todo: This also needs to be when the first profile is created
        if let Some(active_profile) = cfg.active_profile() {
            let mod_objs = Self::get_profile_mod_objs(
                &cfg.profiles(),
                &active_profile,
                mod_list_models.as_ref().borrow_mut(),
            );
            mod_list_model.extend_from_slice(&mod_objs);
        }

        let button_save_mod_list = imp.button_save_mod_list.get();
        let is_profile_modified = imp.is_profile_modified.clone();

        mod_list.bind_model(Some(mod_list_model), move |obj| {
            let item = obj.downcast_ref::<ModObject>().unwrap();
            let row = adw::SwitchRow::builder().title(item.name()).build();

            item.bind_property("enabled", &row, "active")
                .bidirectional()
                .sync_create()
                .build();

            // Show Apply button if not visible already
            item.connect_enabled_notify(clone!(
                #[weak]
                cfg,
                #[weak]
                button_save_mod_list,
                #[weak]
                is_profile_modified,
                move |_| {
                    if !button_save_mod_list.is_visible() && button_save_mod_list.is_sensitive() {
                        button_save_mod_list.set_visible(true);
                    }

                    is_profile_modified
                        .as_ref()
                        .borrow_mut()
                        .entry(cfg.active_profile().unwrap())
                        .and_modify(|b| *b = true)
                        .or_insert(true);
                }
            ));

            row.into()
        });
    }

    fn get_profile_mod_objs(
        profiles: &ModProfiles,
        active: impl AsRef<str>,
        mut mods_store: RefMut<HashMap<String, Vec<ModObject>>>,
    ) -> Vec<ModObject> {
        let mods = profiles.get_profile(active.as_ref()).unwrap();
        let mod_objs = mods
            .mods
            .into_iter()
            .map(|it| ModObject::new(it))
            .collect_vec();
        mods_store.insert(active.as_ref().to_owned(), mod_objs.clone());

        mod_objs
    }

    pub fn present_profile_new_dialog(&self) {
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

fn mod_objs_to_mods(mod_objs: &[ModObject]) -> Mods {
    Mods {
        mods: mod_objs
            .iter()
            .map(|it| it.imp().inner.clone().into_inner().0)
            .collect_vec(),
    }
}
