use crate::{
    app::{App, Message, State},
    games,
};

pub fn show(ui: &mut egui::Ui, url: &str, app: &App) {
    ui.label("Finished");

    if ui.button("Copy url to clipboard").clicked() {
        if let Err(e) = arboard::Clipboard::new().and_then(|mut c| c.set_text(url)) {
            app.message_tx
                .send(Message::GoTo(State::Error(e.to_string())))
                .unwrap();
        } else {
            app.message_tx
                .send(Message::Toast(egui_notify::Toast::success("Copied")))
                .unwrap();
        }
    }

    let import_url = match app.game {
        games::Game::Hsr => "https://stardb.gg/warp-import",
        games::Game::Gi => "https://stardb.gg/genshin/wish-import",
        games::Game::Zzz => "https://stardb.gg/zzz/signal-import",
    };

    ui.hyperlink_to("Click here to import", import_url);

    if ui.button("Sync to stardb").clicked() {
        let import_url = match app.game {
            games::Game::Hsr => "https://stardb.gg/api/warps-import",
            games::Game::Gi => "https://stardb.gg/api/gi/wishes-import",
            games::Game::Zzz => "https://stardb.gg/api/zzz/signals-import",
        };

        let request = if let Some(user) = &app.user {
            ureq::post(import_url).set("Cookie", &user.id)
        } else {
            ureq::post(import_url)
        };

        match request.send_json(serde_json::json!({"url": url})) {
            Ok(r) => {
                app.message_tx
                    .send(Message::Toast(egui_notify::Toast::success(format!(
                        "Synced uid {}",
                        r.into_json::<serde_json::Value>().unwrap()["uid"]
                    ))))
                    .unwrap();
                app.message_tx
                    .send(Message::Toast(egui_notify::Toast::success("Copied")))
                    .unwrap();
            }
            Err(e) => {
                app.message_tx
                    .send(Message::Toast(egui_notify::Toast::error(format!(
                        "Error: {e}"
                    ))))
                    .unwrap();
            }
        }
    }
}
