use std::borrow::BorrowMut;
use std::cell::{Cell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use color_eyre::eyre::Result;
use gtk::gio::ListStore;
use gtk::glib::clone;
use gtk::{gio, glib, SingleSelection, StringObject};
use itertools::Itertools;
use noitad_lib::config::Config;
use noitad_lib::defines::APP_CONFIG_PATH;
use noitad_lib::noita::mod_config::Mods;
use noitad_lib::noita::{GamePath, NoitaPath};
use tracing::{debug, error, info};

use crate::application::NoitadApplication;
use crate::config::{APP_ID, PROFILE};
use crate::objects;
use crate::objects::config::ModProfiles;
use crate::objects::noita_mod::ModObject;
use crate::widgets::game_path_pref::GamePathPreference;

mod imp {
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    use crate::{objects::config::ConfigObject, widgets::game_path_pref::GamePathPreference};

    use super::*;

    #[derive(Debug, gtk::CompositeTemplate, better_default::Default)]
    #[template(resource = "/io/github/nozwock/noitad/ui/window.ui")]
    pub struct NoitadApplicationWindow {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub mod_list_page: TemplateChild<adw::NavigationPage>,

        #[template_child]
        pub setup_game_path_pref: TemplateChild<GamePathPreference>,
        #[template_child]
        pub button_end_setup: TemplateChild<gtk::Button>,

        #[template_child]
        pub profiles_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub mod_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub button_save_mod_list: TemplateChild<gtk::Button>,

        #[default(gio::Settings::new(APP_ID))]
        pub settings: gio::Settings,
        pub config: ConfigObject,
        pub is_initial_setup_done: Rc<RefCell<Option<bool>>>,

        pub mod_list_models: Rc<RefCell<HashMap<String, Vec<ModObject>>>>,
        pub is_profile_modified: Rc<RefCell<HashMap<String, bool>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NoitadApplicationWindow {
        const NAME: &'static str = "NoitadApplicationWindow";
        type Type = super::NoitadApplicationWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            GamePathPreference::ensure_type();

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

            match self.is_initial_setup_done.as_ref().borrow().to_owned() {
                // Initial setup was started but not completed, skip default serialization
                Some(false) => {}
                _ => {
                    _ = dbg!(dbg!(self.config.into_simple_config()).store());
                }
            }

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

        // Setup welcoming screen
        let stack = imp.stack.get();
        self.setup_welcome_page();

        if APP_CONFIG_PATH.is_file() {
            stack.set_visible_child_name("main_page");
        } else {
            imp.is_initial_setup_done
                .as_ref()
                .borrow_mut()
                .replace(false); // Starting initial setup
        }

        // todo: Make main mod list content page collapse when we are on the 'status_no_profile' page in the sidebar
        // Also, make a handler for button_create_first_profile

        let mod_list_model = gio::ListStore::new::<ModObject>();
        self.setup_profile_sidebar(&mod_list_model);
        self.setup_mod_list(&mod_list_model);
    }

    fn setup_welcome_page(&self) {
        let imp = self.imp();
        let setup_game_path_pref = imp.setup_game_path_pref.get();
        let button_end_setup = imp.button_end_setup.get();
        let dropdown_game_path_lookup = setup_game_path_pref.imp().game_path_lookup.get();

        let is_steam_lookup_valid = Rc::new(Cell::new(false));

        macro_rules! validate_initial_setup_callback {
            () => {{
                clone!(
                    #[weak]
                    setup_game_path_pref,
                    #[weak]
                    button_end_setup,
                    #[strong]
                    is_steam_lookup_valid,
                    move |_| {
                        validate_initial_setup(
                            &setup_game_path_pref,
                            &button_end_setup,
                            &is_steam_lookup_valid,
                        );
                    }
                )
            }};
        }

        dropdown_game_path_lookup.connect_selected_item_notify(validate_initial_setup_callback!());
        dropdown_game_path_lookup.notify("selected-item");

        setup_game_path_pref.connect_game_root_path_notify(validate_initial_setup_callback!());
        #[cfg(target_os = "linux")]
        setup_game_path_pref.connect_wine_prefix_path_notify(validate_initial_setup_callback!());

        fn validate_initial_setup(
            obj: &GamePathPreference,
            finish_button: &gtk::Button,
            is_steam_lookup_valid: &Rc<Cell<bool>>,
        ) {
            let is_valid = match obj
                .imp()
                .game_path_lookup
                .selected_item()
                .and_downcast::<StringObject>()
                .unwrap()
                .string()
                .to_lowercase()
                .as_str()
            {
                "steam" => {
                    is_steam_lookup_valid.get()
                        || if let NoitaPath::Steam = NoitaPath::default() {
                            is_steam_lookup_valid.set(true);
                            true
                        } else {
                            // todo: show error via toast
                            false
                        }
                }
                "manual" => {
                    obj.game_root_path().is_some()
                        && (cfg!(not(target_os = "linux")) || obj.wine_prefix_path().is_some())
                }
                _ => unreachable!(),
            };

            finish_button.set_sensitive(is_valid);
        }

        let stack = imp.stack.get();
        let config = imp.config.clone();
        let is_initial_setup_done = imp.is_initial_setup_done.clone();
        button_end_setup.connect_clicked(move |_| {
            let kind = setup_game_path_pref
                .imp()
                .game_path_lookup
                .selected_item()
                .and_downcast::<StringObject>()
                .unwrap()
                .string()
                .to_lowercase();

            let noita_path = match kind.as_str() {
                "steam" => NoitaPath::Steam,
                "manual" => NoitaPath::Other(Some(GamePath {
                    game_root: setup_game_path_pref.game_root_path().unwrap(),
                    wine_prefix: setup_game_path_pref.wine_prefix_path(),
                })),
                _ => unreachable!(),
            };

            info!(?noita_path, "Completing initial setup");

            config.set_noita_path(objects::config::NoitaPath(noita_path));

            stack.set_visible_child_name("main_page");
            is_initial_setup_done.as_ref().borrow_mut().replace(true);
        });
    }

    // todo: Currently, whatever profile you're viewing becomes the active_profile
    // but it shouldn't be like that. This needs to be decoupled, a dropdown in preferences
    // for setting active profile and a sidebar list for viewing a profile
    fn setup_profile_sidebar(&self, mod_list_model: &ListStore) {
        let imp = self.imp();
        let cfg = &imp.config;

        let profiles_list = imp.profiles_list.get();
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

        // todo: Reset ListBox::move-cursor, so that moving keys only sets focus and not select row aswell,
        // and remove/unbind ListBox::toggle-cursor-row, since toggling doesn't makes sense here
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
        let selection_model = SingleSelection::new(Some(profiles_model));
        profiles_list.bind_model(
            Some(&selection_model),
            clone!(
                #[weak]
                cfg,
                #[upgrade_or_panic]
                move |obj| {
                    let profile = obj.downcast_ref::<StringObject>().unwrap();

                    let row = adw::ActionRow::builder()
                        .title(profile.string().as_str())
                        .build();
                    let default_profile = gtk::Image::builder()
                        .icon_name("emblem-default-symbolic")
                        .tooltip_text("Default Profile")
                        .visible(false)
                        .build();

                    // fix(done): disable remove action for default profile
                    // Leaving it here for this one commit...
                    //
                    // This doesn't seem to work, disabling menu item by disabling its associated action that is
                    // row.action_set_enabled("profile-row.set-default", false);
                    //
                    // Others say that just there being an invalid action name or maybe just None(?), should make
                    // the menu item button insensitive but ofcourse it DOESN'T WORK!
                    //
                    // So I've just decided for now to disable the whole popover_button altogther for the default profile row.
                    // That works in our case...
                    //
                    // So it turns out, this ActionEntry is some weird API to create Actions quickly
                    // and so set_enabled(), etc methods don't exist for it and instead we have to get
                    // action from the SimpleActionGroup via the lookup_action() method
                    //
                    // Since I've settled for disabling the popover button instead, but incase I ever want to disable the action
                    // then here it is...
                    // ```
                    // action_group
                    //     .lookup_action("set-default")
                    //     .unwrap()
                    //     .downcast::<gio::SimpleAction>()
                    //     .unwrap()
                    //     .set_enabled(false);
                    // ```

                    let menu_model = gio::Menu::new();
                    menu_model.append_item(&gio::MenuItem::new(
                        Some("Set as Default"),
                        Some("profile-row.set-default"),
                    ));
                    menu_model.append_item(&gio::MenuItem::new(
                        Some("Remove Profile"),
                        Some("profile-row.remove"),
                    ));
                    let popover_button = gtk::MenuButton::builder()
                        .valign(gtk::Align::Center)
                        .icon_name("view-more-symbolic")
                        .css_classes(["flat"])
                        .menu_model(&menu_model)
                        .build();

                    let action_group = gio::SimpleActionGroup::new();

                    // todo: Only have action defined once with getting state (profile name) via action parameter
                    // Also, attach them to window, that way it seems action.set_enabled method will work

                    let set_default = gio::ActionEntry::builder("set-default")
                        .activate(clone!(
                            #[weak]
                            profile,
                            #[weak]
                            cfg,
                            move |_, _, _| {
                                cfg.set_active_profile(Some(profile.string()));
                            }
                        ))
                        .build();
                    let remove_profile = gio::ActionEntry::builder("remove")
                        .activate(clone!(
                            #[weak]
                            profile,
                            #[weak]
                            cfg,
                            move |_, _, _| {
                                let mut profiles = cfg.profiles();
                                _ = profiles
                                    .remove_profile(profile.string().as_str())
                                    .inspect_err(|err| error!(%err));
                                cfg.set_profiles(profiles);
                            }
                        ))
                        .build();
                    action_group.add_action_entries([set_default, remove_profile]);
                    row.insert_action_group("profile-row", Some(&action_group));

                    row.add_suffix(&default_profile);
                    row.add_suffix(&popover_button);

                    row.into()
                }
            ),
        );

        let mod_list_page = imp.mod_list_page.get();
        profiles_list.connect_row_selected(clone!(
            #[weak]
            cfg,
            #[weak]
            mod_list_model,
            #[weak]
            mod_list_models,
            #[weak]
            mod_list_page,
            move |_obj, row| {
                let active_profile = row
                    .unwrap() // fix: can be None when user deletes selected profile
                    .downcast_ref::<adw::ActionRow>()
                    .unwrap()
                    .title();
                mod_list_page.set_title(&format!("Profile — {}", active_profile.as_str()));

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
                // todo: This also needs to be when the first profile is created
                mod_list_model.extend_from_slice(&mod_objs);
            }
        ));

        // todo: Select the last selected profile by default instead of default_profile, stored in GSettings at window exit
        if let Some(active_profile) = cfg.active_profile() {
            profiles_list.select_row(
                profiles_list
                    .row_at_index(
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
                            .unwrap() as i32,
                    )
                    .as_ref(),
            );
        }

        // Set the default profile icon
        cfg.connect_active_profile_notify(clone!(
            #[weak]
            profiles_list,
            #[weak]
            selection_model,
            move |cfg| {
                for i in 0..selection_model.n_items() {
                    let row = profiles_list
                        .row_at_index(i as i32)
                        .unwrap()
                        .downcast::<adw::ActionRow>()
                        .unwrap();

                    let image = row
                        .child() // root Box
                        .unwrap()
                        .last_child() // Box for suffix children
                        .unwrap()
                        .observe_children()
                        .into_iter()
                        .map(|it| it.unwrap())
                        .find_map(|child| child.downcast::<gtk::Image>().ok())
                        .unwrap();

                    let popover_button = row
                        .child() // root Box
                        .unwrap()
                        .last_child() // Box for suffix children
                        .unwrap()
                        .observe_children()
                        .into_iter()
                        .map(|it| it.unwrap())
                        .find_map(|child| child.downcast::<gtk::MenuButton>().ok())
                        .unwrap();

                    if cfg.active_profile().unwrap() == row.title().as_str() {
                        image.set_visible(true);
                        popover_button.set_sensitive(false);
                    } else {
                        image.set_visible(false);
                        popover_button.set_sensitive(true);
                    }
                }
            }
        ));
        cfg.notify_active_profile(); // To set the icon immediately
    }

    fn setup_mod_list(&self, mod_list_model: &ListStore) {
        let imp = self.imp();
        let cfg = imp.config.clone();

        let mod_list = imp.mod_list.get();

        // todo: Aside from the sync for the active profile with noita mod_config,
        // there needs to be sync for others where new mod entries are added in those profiles
        // when they're being loaded

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

        let button_save_mod_list = imp.button_save_mod_list.get();
        button_save_mod_list.connect_clicked(clone!(
            #[weak]
            imp,
            move |btn| {
                btn.set_sensitive(false);

                let is_profile_modified = imp.is_profile_modified.clone();
                let mod_list_models = imp.mod_list_models.clone();
                let mod_list_models_ref = mod_list_models.as_ref().borrow();
                let mut profiles = imp.config.profiles();
                is_profile_modified
                    .as_ref()
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
