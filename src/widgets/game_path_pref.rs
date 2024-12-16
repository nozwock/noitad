use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{
    gio,
    glib::{self, clone},
    StringObject,
};
use noitad_lib::noita::{GamePath, NoitaPath};
use tracing::error;

mod imp {
    use std::{cell::RefCell, path::PathBuf, rc::Rc};

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nozwock/noitad/ui/game_path_pref.ui")]
    pub struct GamePathPreference {
        #[template_child]
        pub game_path_lookup: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub revealer_manual_lookup: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub row_game_root_location: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub button_game_root_location: TemplateChild<gtk::Button>,
        #[template_child]
        pub row_wine_prefix_location: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub button_wine_prefix_location: TemplateChild<gtk::Button>,

        pub game_root_path: Rc<RefCell<Option<PathBuf>>>,
        pub wine_prefix_path: Rc<RefCell<Option<PathBuf>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GamePathPreference {
        const NAME: &'static str = "GamePathPreference";
        type Type = super::GamePathPreference;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GamePathPreference {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.setup_ui();
        }
    }
    impl WidgetImpl for GamePathPreference {}
    impl BoxImpl for GamePathPreference {}
}

glib::wrapper! {
    pub struct GamePathPreference(ObjectSubclass<imp::GamePathPreference>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl GamePathPreference {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();
        let game_path_lookup = imp.game_path_lookup.get();
        let revealer_manual_lookup = imp.revealer_manual_lookup.get();

        game_path_lookup
            .bind_property("selected-item", &revealer_manual_lookup, "reveal-child")
            .transform_to(|_, obj: StringObject| {
                Some(match obj.string().to_lowercase().as_str() {
                    "steam" => false,
                    "manual" => true,
                    _ => unreachable!(),
                })
            })
            .sync_create()
            .build();

        macro_rules! setup_pick_folder_row {
            ($row:ident, $button:ident, $state:ident, $validation_fn:expr) => {{
                $button.connect_clicked(clone!(
                    #[weak]
                    imp,
                    move |_| {
                        let state = imp.$state.clone();
                        let row = imp.$row.clone();

                        imp.obj().pick_dir(move |path| {
                            match path {
                                Some(path) => {
                                    if ($validation_fn)(&path) {
                                        row.remove_css_class("error");
                                        row.add_css_class("success");
                                        row.set_subtitle(&path.to_string_lossy());
                                        *state.as_ref().borrow_mut() = Some(path);
                                    } else if state.as_ref().borrow().is_none() {
                                        error!(?path, "Invalid");
                                        row.add_css_class("error");
                                    }
                                }
                                None => {
                                    row.remove_css_class("error");
                                }
                            };
                        });
                    }
                ));
            }};
        }

        let button_game_root_location = imp.button_game_root_location.get();
        let button_wine_prefix_location = imp.button_wine_prefix_location.get();

        setup_pick_folder_row!(
            row_game_root_location,
            button_game_root_location,
            game_root_path,
            |path: &PathBuf| { path.join("mods").is_dir() }
        );

        setup_pick_folder_row!(
            row_wine_prefix_location,
            button_wine_prefix_location,
            wine_prefix_path,
            |path: &PathBuf| {
                NoitaPath::Other(Some(GamePath {
                    wine_prefix: Some(path.into()),
                    ..Default::default()
                }))
                .save_dir()
                .is_some()
            }
        );
    }

    fn pick_dir(&self, callback: impl FnOnce(Option<PathBuf>) + 'static) {
        let dialog = gtk::FileDialog::new();
        dialog.select_folder(
            self.root().and_downcast_ref::<adw::ApplicationWindow>(),
            None::<&gio::Cancellable>,
            |file| {
                callback(
                    file.inspect_err(|err| error!(%err))
                        .ok()
                        .map(|it| it.path())
                        .flatten(),
                );
            },
        );
    }
}
