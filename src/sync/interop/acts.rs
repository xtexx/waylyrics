use glib_macros::clone;
use gtk::{
    gio::SimpleAction,
    glib::{self, VariantTy},
    prelude::*,
    subclass::prelude::ObjectSubclassIsExt,
    Application, NamedAction, Shortcut, ShortcutController, ShortcutTrigger,
};
use tracing::{error, info, warn};

use crate::{
    app,
    lyric::{default_search_query, LyricOwned},
    sync::{
        interop::reset_lyric_labels, lyric::cache::update_lyric_cache, search_window, TrackState,
        LYRIC, PLAYER, PLAYER_FINDER, TRACK_PLAYING_STATE,
    },
};

pub fn register_action_disconnect(app: &Application) {
    let action = SimpleAction::new("disconnect", None);
    action.connect_activate(|_, _| {
        PLAYER.set(None);
    });
    app.add_action(&action);
}

pub fn register_sigusr1_disconnect() {
    glib::unix_signal_add_local(libc::SIGUSR1, move || {
        PLAYER.set(None);
        Continue(true)
    });
}

// TODO: code cleanup
pub fn register_action_search_lyric(app: &Application, wind: &app::Window, trigger: &str) {
    let action = SimpleAction::new("search-lyric", None);
    let cache_lyrics = wind.imp().cache_lyrics.get();
    action.connect_activate(move |_, _| {
        // Get current playing track
        let query_default = TRACK_PLAYING_STATE.with_borrow(|TrackState { metainfo, .. }| {
            metainfo.as_ref().map(|track| {
                default_search_query(
                    track.meta.album_name().unwrap_or_default(),
                    &track.meta.artists().unwrap_or_default(),
                    &track.title,
                )
            })
        });

        let window = search_window::Window::new(query_default.as_deref(), cache_lyrics);
        window.present();
    });
    app.add_action(&action);

    let shortcut = Shortcut::builder()
        .action(&NamedAction::new("app.search-lyric"))
        .trigger(&ShortcutTrigger::parse_string(trigger).unwrap())
        .build();
    let controller = ShortcutController::new();
    controller.set_scope(gtk::ShortcutScope::Global);
    controller.add_shortcut(shortcut);
    wind.add_controller(controller);
}

pub fn register_action_reload_lyric(app: &Application, wind: &app::Window, trigger: &str) {
    let action = SimpleAction::new("reload-lyric", None);
    action.connect_activate(move |_, _| {
        TRACK_PLAYING_STATE.take();
        info!("cleaned lyric");
    });
    app.add_action(&action);

    let shortcut = Shortcut::builder()
        .action(&NamedAction::new("app.reload-lyric"))
        .trigger(&ShortcutTrigger::parse_string(trigger).unwrap())
        .build();
    let controller = ShortcutController::new();
    controller.set_scope(gtk::ShortcutScope::Global);
    controller.add_shortcut(shortcut);
    wind.add_controller(controller);
}

pub fn register_action_remove_lyric(app: &Application, wind: &app::Window) {
    let action = SimpleAction::new("remove-lyric", None);
    let cache_lyrics = wind.imp().cache_lyrics.get();
    action.connect_activate(clone!(@weak wind as window => move |_, _| {
        // Clear current lyric
        LYRIC.with_borrow_mut(|(origin, translation)| {
            *origin = LyricOwned::LineTimestamp(vec![]);
            *translation = LyricOwned::None;
        });
        // Update cache
        if cache_lyrics {
            TRACK_PLAYING_STATE.with_borrow(|TrackState{ cache_path, ..}| {
                if let Some(cache_path) = cache_path {
                    update_lyric_cache(cache_path);
                }
            });
        }
        // Remove current lyric inside window
        reset_lyric_labels(&window);
        info!("removed lyric");
    }));
    app.add_action(&action);
}

pub fn register_action_connect(app: &Application) {
    let connect = SimpleAction::new("connect", Some(VariantTy::STRING));
    connect.connect_activate(|_, player_id| {
        let Some(player_id) = player_id.and_then(|p| p.str()) else {
            warn!("did not received string paramter for action \'app.connect\'");
            return;
        };
        PLAYER_FINDER.with_borrow(|player_finder| {
            if let Ok(player) = player_finder.find_by_name(player_id) {
                PLAYER.set(Some(player));
            } else {
                error!("cannot connect to: {player_id}");
            }
        });
    });
    app.add_action(&connect);
}
