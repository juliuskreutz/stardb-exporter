use std::thread;

use crate::{
    app::{App, Message, State},
    games,
};

pub fn show(ui: &mut egui::Ui, achievements: &[u32], app: &App) {
    let key = match app.game {
        games::Game::Hsr => "hsr_achievements",
        games::Game::Gi => "gi_achievements",
        _ => unimplemented!(),
    };

    ui.label("Finished");

    if ui
        .button(format!(
            "Copy {} achievements to clipboard",
            achievements.len()
        ))
        .clicked()
    {
        if let Err(e) = arboard::Clipboard::new()
            .and_then(|mut c| c.set_text(serde_json::json!({ key: achievements }).to_string()))
        {
            app.message_tx
                .send(Message::GoTo(State::Error(e.to_string())))
                .unwrap();
        } else {
            app.message_tx
                .send(Message::Toast(egui_notify::Toast::success("Copied")))
                .unwrap();
        }
    }

    ui.hyperlink_to("Click here to import", "https://stardb.gg/import");

    if let Some(user) = &app.user
        && ui
            .button(format!("Sync to account: \"{}\"", user.username))
            .clicked()
    {
        app.message_tx
            .send(Message::Toast(egui_notify::Toast::info("Syncing")))
            .unwrap();

        let prefix = match app.game {
            games::Game::Hsr => "",
            games::Game::Gi => "gi/",
            _ => unimplemented!(),
        };

        let url = format!("https://stardb.gg/api/users/me/{prefix}achievements/completed");

        {
            let message_tx = app.message_tx.clone();
            let id = user.id.clone();
            let achievements = achievements.to_vec();

            thread::spawn(move || {
                let to_delete: Vec<i32> = match ureq::get(&url).header("Cookie", &id).call() {
                    Ok(r) => {
                        if r.status() == 200 {
                            r.into_body().read_json().unwrap()
                        } else {
                            message_tx
                                .send(Message::Toast(egui_notify::Toast::error(
                                    "Error. Try Relogging",
                                )))
                                .unwrap();

                            return;
                        }
                    }
                    Err(e) => {
                        message_tx
                            .send(Message::Toast(egui_notify::Toast::error(format!(
                                "Error: {e}"
                            ))))
                            .unwrap();
                        return;
                    }
                };

                match ureq::delete(&url)
                    .header("Cookie", &id)
                    .force_send_body()
                    .send_json(to_delete)
                {
                    Ok(r) => {
                        if r.status() != 200 {
                            message_tx
                                .send(Message::Toast(egui_notify::Toast::error(
                                    "Error. Try Relogging",
                                )))
                                .unwrap();

                            return;
                        }
                    }
                    Err(e) => {
                        message_tx
                            .send(Message::Toast(egui_notify::Toast::error(format!(
                                "Error: {e}"
                            ))))
                            .unwrap();
                        return;
                    }
                };

                match ureq::put(&url)
                    .header("Cookie", &id)
                    .send_json(achievements)
                {
                    Ok(r) => {
                        if r.status() == 200 {
                            message_tx
                                .send(Message::Toast(egui_notify::Toast::success("Synced")))
                                .unwrap();
                        } else {
                            message_tx
                                .send(Message::Toast(egui_notify::Toast::error(
                                    "Error. Try Relogging",
                                )))
                                .unwrap();
                        }
                    }
                    Err(e) => {
                        message_tx
                            .send(Message::Toast(egui_notify::Toast::error(format!(
                                "Error: {e}"
                            ))))
                            .unwrap();
                    }
                }
            });
        }
    }
}
