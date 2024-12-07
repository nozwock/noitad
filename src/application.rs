use gettextrs::gettext;
use tracing::{debug, info};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};

use crate::config::{APP_ID, PKGDATADIR, PROFILE, VERSION};
use crate::window::NoitadApplicationWindow;

mod imp {
    use super::*;
    use glib::WeakRef;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct NoitadApplication {
        pub window: OnceCell<WeakRef<NoitadApplicationWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NoitadApplication {
        const NAME: &'static str = "NoitadApplication";
        type Type = super::NoitadApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for NoitadApplication {}

    impl ApplicationImpl for NoitadApplication {
        fn activate(&self) {
            debug!("GtkApplication<NoitadApplication>::activate");
            self.parent_activate();
            let app = self.obj();

            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            let window = NoitadApplicationWindow::new(&app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            app.main_window().present();
        }

        fn startup(&self) {
            debug!("GtkApplication<NoitadApplication>::startup");
            self.parent_startup();
            let app = self.obj();

            // Set icons for shell
            gtk::Window::set_default_icon_name(APP_ID);

            app.setup_css();
            app.setup_gactions();
            app.setup_accels();
        }
    }

    impl GtkApplicationImpl for NoitadApplication {}
    impl AdwApplicationImpl for NoitadApplication {}
}

glib::wrapper! {
    pub struct NoitadApplication(ObjectSubclass<imp::NoitadApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl NoitadApplication {
    fn main_window(&self) -> NoitadApplicationWindow {
        self.imp().window.get().unwrap().upgrade().unwrap()
    }

    fn setup_gactions(&self) {
        // Quit
        let action_quit = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| {
                // This is needed to trigger the delete event and saving the window state
                app.main_window().close();
                app.quit();
            })
            .build();

        // About
        let action_about = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about_dialog();
            })
            .build();

        let action_profile_new = gio::ActionEntry::builder("profile-new")
            .activate(|app: &Self, _, _| {
                app.main_window().profile_new();
            })
            .build();

        self.add_action_entries([action_quit, action_about, action_profile_new]);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("window.close", &["<Control>w"]);
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/io/github/nozwock/noitad/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::builder()
            .application_icon(APP_ID)
            .application_name("Noitad")
            .developer_name("nozwock")
            .version(VERSION)
            .issue_url("https://github.com/nozwock/noitad/issues")
            .translator_credits(gettext("translator-credits"))
            .build();

        dialog.present(Some(&self.main_window()));
    }

    pub fn run(&self) -> glib::ExitCode {
        info!("Noitad ({})", APP_ID);
        info!("Version: {} ({})", VERSION, PROFILE);
        info!("Datadir: {}", PKGDATADIR);

        ApplicationExtManual::run(self)
    }
}

impl Default for NoitadApplication {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", APP_ID)
            .property("resource-base-path", "/io/github/nozwock/noitad/")
            .build()
    }
}
