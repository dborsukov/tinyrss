use crate::worker::{Channel, Item};
use chrono::{Duration, Local, TimeZone, Utc};
use eframe::epaint::text::{LayoutJob, TextWrapping};
use egui::{Frame, Hyperlink, Label, RichText, TextFormat};
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

pub fn channel_card(ui: &mut egui::Ui, channel: &Channel, search: &str) {
    let mut show = true;
    if let Some(title) = &channel.title {
        if !title.to_lowercase().contains(&search.to_lowercase()) && !search.is_empty() {
            show = false;
        }
    }
    if show {
        Frame {
            fill: egui::Color32::from_rgb(0, 50, 0),
            inner_margin: egui::Margin::same(4.0),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(title) = &channel.title {
                ui.label(RichText::new(title).strong().heading());
            } else {
                ui.label(RichText::new("<no title>").strong().heading());
            }
        });
    }
}

pub fn feed_card(ui: &mut egui::Ui, item: &Item) {
    Frame {
        fill: egui::Color32::from_rgb(0, 50, 0),
        inner_margin: egui::Margin::same(4.0),
        ..Default::default()
    }
    .show(ui, |ui| {
        ui.set_width(ui.available_width());
        if let Some(title) = &item.title {
            let mut job = LayoutJob::single_section(title.to_string(), TextFormat::default());
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
            ui.add_space(5.);
            if let Some(channel_title) = &item.channel_title {
                ui.label(truncate(channel_title, 25, None));
            }
        });
    });
}
