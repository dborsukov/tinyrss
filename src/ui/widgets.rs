use super::THEME;
use crate::worker::{Channel, Item, ToWorker};
use chrono::{Duration, Local, TimeZone, Utc};
use crossbeam_channel::Sender;
use eframe::epaint::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Button, CollapsingHeader, FontId, Frame, Hyperlink, Label, Layout, RichText, TextFormat,
    Vec2,
};
use unicode_truncate::UnicodeTruncateStr;

pub fn truncate(string: &str, width: usize, trim_char: Option<&str>) -> String {
    let (truncated, width_t) = string.unicode_truncate(width);
    let mut truncated_string = truncated.to_string();
    if width == width_t {
        truncated_string.push_str(trim_char.unwrap_or("…"));
    }
    truncated_string
}

pub fn timestamp_to_human_readable(timestamp: i64) -> String {
    let dt = match Utc.timestamp_millis_opt(timestamp * 1000).earliest() {
        Some(dt) => dt,
        None => return String::from("???"),
    };

    let duration = Duration::seconds(Local::now().timestamp() - timestamp);

    if duration.num_minutes() < 60 {
        if duration.num_minutes() == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", duration.num_minutes())
        }
    } else if duration.num_hours() < 24 {
        if duration.num_hours() == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", duration.num_hours())
        }
    } else if duration.num_days() < 7 {
        if duration.num_days() == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", duration.num_days())
        }
    } else if duration.num_weeks() < 4 {
        if duration.num_weeks() == 1 {
            "1 week ago".to_string()
        } else {
            format!("{} weeks ago", duration.num_weeks())
        }
    } else {
        dt.format("%d %b %Y").to_string()
    }
}

pub fn channel_card(
    ui: &mut egui::Ui,
    sender: Option<Sender<ToWorker>>,
    channel: &Channel,
    search: &str,
) {
    let mut show = true;
    if let Some(title) = &channel.title {
        if !title.to_lowercase().contains(&search.to_lowercase()) && !search.is_empty() {
            show = false;
        }
    }
    if show {
        Frame {
            fill: THEME.colors.bg,
            rounding: THEME.rounding.large,
            inner_margin: egui::Margin::same(6.0),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(title) = &channel.title {
                CollapsingHeader::new(RichText::new(truncate(title, 40, None)).strong().heading())
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.spacing_mut().button_padding = Vec2::new(6., 3.);
                        ui.add_space(THEME.spacing.small);
                        if let Some(description) = &channel.description {
                            ui.add(Label::new(RichText::new(description)).wrap(true));
                            ui.add_space(THEME.spacing.medium);
                        }
                        if ui
                            .add(Button::new("Unsubscribe").fill(THEME.colors.warning))
                            .clicked()
                        {
                            if let Some(sender) = sender {
                                sender
                                    .send(ToWorker::Unsubscribe {
                                        id: channel.id.clone(),
                                    })
                                    .unwrap();
                            }
                        }
                    });
            } else {
                ui.label(RichText::new("<no title>").strong().heading());
            }
        });
    }
}

pub fn feed_card(ui: &mut egui::Ui, sender: Option<Sender<ToWorker>>, item: &Item) {
    Frame {
        fill: THEME.colors.bg,
        rounding: THEME.rounding.large,
        inner_margin: egui::Margin::same(6.0),
        ..Default::default()
    }
    .show(ui, |ui| {
        ui.set_width(ui.available_width());
        if let Some(title) = &item.title {
            let mut job = LayoutJob::single_section(
                title.to_string(),
                TextFormat {
                    font_id: FontId::proportional(22.0),
                    ..Default::default()
                },
            );
            job.wrap = TextWrapping {
                max_rows: 1,
                break_anywhere: true,
                overflow_character: Some('…'),
                ..Default::default()
            };
            ui.add(Hyperlink::from_label_and_url(job, &item.link));
        } else {
            ui.add(Label::new(RichText::new("<no title>")));
        }
        ui.horizontal(|ui| {
            ui.label(timestamp_to_human_readable(item.published));
            ui.label("·");
            if let Some(channel_title) = &item.channel_title {
                ui.label(truncate(channel_title, 30, None));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if item.dismissed {
                    if ui.link("Restore").clicked() {
                        if let Some(sender) = sender {
                            sender
                                .send(ToWorker::SetDismissed {
                                    id: item.id.clone(),
                                    dismissed: false,
                                })
                                .unwrap();
                        }
                    }
                } else {
                    if ui.link("Dismiss").clicked() {
                        if let Some(sender) = sender {
                            sender
                                .send(ToWorker::SetDismissed {
                                    id: item.id.clone(),
                                    dismissed: true,
                                })
                                .unwrap();
                        }
                    }
                }
            });
        });
    });
}
