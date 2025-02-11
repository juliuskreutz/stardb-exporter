use std::{sync::mpsc, thread};

use crate::app::{App, Message, State, User};

pub fn show(ui: &mut egui::Ui, username: &str, password: &str, app: &App) {
    let mut username = username.to_string();
    let mut password = password.to_string();

    ui.label("Username:");
    let username_edit = ui.text_edit_singleline(&mut username);

    ui.label("Password:");
    let password_edit = ui.add(egui::TextEdit::singleline(&mut password).password(true));

    if ui.button("Login").clicked() {
        login(&username, &password, &app.message_tx);

        app.message_tx
            .send(Message::GoTo(State::Waiting("Loggin In".to_string())))
            .unwrap();
    } else if username_edit.changed() || password_edit.changed() {
        app.message_tx
            .send(Message::GoTo(State::Login(username, password)))
            .unwrap();
    }
}

fn login(username: &str, password: &str, message_tx: &mpsc::Sender<Message>) {
    let username = username.to_string();
    let password = password.to_string();
    let message_tx = message_tx.clone();

    thread::spawn(move || {
        let json = serde_json::json!({
            "username": username,
            "password": password
        });

        let id = ureq::post("https://stardb.gg/api/users/auth/login")
            .send_json(json)
            .ok()
            .and_then(|r| {
                r.headers()
                    .get("Set-Cookie")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|id| id.split(';').next())
                    .map(|s| s.to_string())
            });

        if let Some(id) = id {
            let username = username.to_string();

            let user = User { id, username };

            message_tx.send(Message::User(Some(user))).unwrap();
            message_tx.send(Message::GoTo(State::Menu)).unwrap();
        } else {
            message_tx
                .send(Message::GoTo(State::Error(
                    "There was an error during the login".to_string(),
                )))
                .unwrap();
        }
    });
}
