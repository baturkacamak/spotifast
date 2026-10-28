//! The Jam this computer hosts, in the right panel.

use egui::{Frame, Margin, Sense, Vec2};

use crate::app::App;
use crate::i18n::{gettext, ngettext};
use crate::jam::{JamSession, Member, State};
use crate::model::Action;
use crate::theme::{self, Icon, Palette};

use super::widgets;

const AVATAR: f32 = 36.0;
const ROW_GAP: f32 = 14.0;

pub fn side_panel(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    let fit = super::yielding_panel(
        ui.ctx(),
        "jam-panel",
        theme::SIDE_PANEL_MIN_WIDTH..=560.0,
        app.settings.queue_width,
        ui.available_width() - super::topbar::least_width(ui.ctx()),
    );
    let panel = egui::Panel::right("jam-panel")
        .resizable(true)
        .default_size(app.settings.queue_width)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(12, 12)),
        );
    let response = panel.show(ui, |ui| {
        let window_controls =
            super::window_controls_reservation(ui.ctx(), true, false, ui.available_width());
        ui.add_space(window_controls.queue_top);
        let mut close = false;
        egui::Sides::new().shrink_left().show(
            ui,
            |ui| {
                ui.add_space(4.0);
                theme::text(ui, gettext(locale, "Jam"), theme::bold(16.0), palette.text);
            },
            |ui| {
                close = theme::icon_button(
                    ui,
                    Icon::X,
                    18.0,
                    palette.secondary,
                    palette.text,
                    &gettext(locale, "Close"),
                )
                .clicked();
            },
        );
        if close {
            app.actions.push(Action::ToggleJamPanel);
        }
        ui.add_space(12.0);
        egui::ScrollArea::vertical()
            .id_salt("jam-panel-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| contents(app, ui));
    });
    // The panel shares the Queue's width, so switching between the two
    // never moves the page.
    let width = response.response.rect.width();
    if (width - app.settings.queue_width).abs() > 1.0
        && super::panel_width_chosen(ui.ctx(), "jam-panel", &fit)
    {
        app.settings.queue_width = width;
        app.actions.push(Action::SettingsChanged);
    }
}

fn contents(app: &mut App, ui: &mut egui::Ui) {
    let state = app.jam.clone();
    let palette = app.palette;
    let locale = app.locale;
    match state {
        State::Idle => {
            theme::text(
                ui,
                gettext(
                    locale,
                    "Start a Jam and share its link. Everyone who joins can add songs to the queue this computer plays.",
                )
                .as_ref(),
                theme::regular(13.0),
                palette.secondary,
            );
            ui.add_space(16.0);
            start_button(app, ui, &palette, &gettext(locale, "Start a Jam"));
        }
        State::Starting => {
            widgets::loading_row(ui, &palette, locale);
            ui.add_space(8.0);
            theme::text(
                ui,
                gettext(locale, "Starting the Jam…"),
                theme::regular(13.0),
                palette.secondary,
            );
        }
        State::Hosting(session) => hosting(app, ui, &palette, &session),
        State::Ending(_) => {
            theme::text(
                ui,
                gettext(locale, "Ending the Jam…"),
                theme::regular(13.0),
                palette.secondary,
            );
        }
        State::Failed(error) => {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                theme::icon(ui, Icon::CircleAlert, 16.0, palette.danger);
                theme::text(
                    ui,
                    widgets::sanitize_error_message(&error, locale),
                    theme::regular(13.0),
                    palette.secondary,
                );
            });
            ui.add_space(16.0);
            start_button(app, ui, &palette, &gettext(locale, "Try again"));
        }
    }
}

fn start_button(app: &mut App, ui: &mut egui::Ui, palette: &Palette, label: &str) {
    if theme::pill_button(ui, palette, label, true).clicked() {
        app.actions.push(Action::StartJam);
    }
}

fn hosting(app: &mut App, ui: &mut egui::Ui, palette: &Palette, session: &JamSession) {
    let locale = app.locale;
    let listening = session.members.len() as u32;
    theme::text(
        ui,
        ngettext(
            locale,
            // Translators: {count} is the number of people in the Jam.
            "{count} person listening",
            "{count} people listening",
            listening,
        )
        .as_ref(),
        theme::regular(13.0),
        palette.secondary,
    );
    ui.add_space(12.0);
    for member in &session.members {
        member_row(app, ui, palette, member);
        ui.add_space(ROW_GAP);
    }
    ui.add_space(4.0);
    let link = if session.uri.is_empty() {
        session.id.clone()
    } else {
        session.uri.clone()
    };
    theme::text(ui, link, theme::regular(12.0), palette.secondary);
    ui.add_space(10.0);
    if theme::pill_button(ui, palette, &gettext(locale, "Copy Jam link"), true).clicked() {
        app.actions.push(Action::CopyJamLink);
    }
    ui.add_space(8.0);
    if theme::pill_button(ui, palette, &gettext(locale, "End the Jam"), false).clicked() {
        app.actions.push(Action::EndJam);
    }
}

fn member_row(app: &App, ui: &mut egui::Ui, palette: &Palette, member: &Member) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(AVATAR), Sense::hover());
        if ui.is_rect_visible(rect) {
            widgets::paint_cover(
                ui,
                palette,
                member.image.as_deref(),
                rect,
                AVATAR / 2.0,
                Icon::User,
                Some(app.backend.art()),
            );
        }
        ui.add_space(4.0);
        theme::text(
            ui,
            member.display_name(),
            theme::regular(14.0),
            palette.text,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_without_a_display_name_shows_their_username() {
        let member = Member {
            username: "alice".into(),
            name: String::new(),
            image: None,
        };
        assert_eq!(member.display_name(), "alice");
    }
}
