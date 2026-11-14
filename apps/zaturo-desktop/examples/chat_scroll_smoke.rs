//! Run in a separate Xvfb display. All rows are synthetic; no hub or credentials are used.
use anyhow::{Result, ensure};
use slint::{Model, ModelRc, VecModel};
use std::{cell::RefCell, rc::Rc, time::Duration};
use zaturo_desktop::{
    api::{FeedItem, FeedPage},
    ui::{AppWindow, FeedRow},
};
fn item(index: i64) -> FeedItem {
    FeedItem {
        id: format!("synthetic-{index:04}"),
        created_at: 1_800_000_000_000 + index * 60_000,
        kind: "chat".into(),
        role: if index % 2 == 0 { "owner" } else { "assistant" }.into(),
        text: format!(
            "Mensagem fictícia {index}. Esta conversa existe apenas para verificar o layout e a posição de leitura, sem rede ou dados reais."
        ),
        task_id: None,
        artifact_id: None,
        sensitive: false,
    }
}
fn page(start: i64, end: i64, more: bool) -> FeedPage {
    let items = (start..end).rev().map(item).collect::<Vec<_>>();
    FeedPage {
        received_count: items.len() as u32,
        items,
        has_more: more,
    }
}
fn apply(ui: &AppWindow, page: FeedPage, append: bool, mode: i32) {
    if let Some(items) = ui.merge_page(page, append, false) {
        ui.prepare_scroll(mode);
        ui.set_events(ModelRc::new(VecModel::from(
            items
                .into_iter()
                .map(|item| FeedRow {
                    id: item.id.into(),
                    time: "12:34".into(),
                    label: if item.role == "owner" {
                        "Você"
                    } else {
                        "ZaturoBot"
                    }
                    .into(),
                    text: item.text.into(),
                    role: item.role.into(),
                    task: "".into(),
                    artifact: "".into(),
                })
                .collect::<Vec<_>>(),
        )));
    } else if mode == 1 {
        ui.scroll_to_bottom();
    }
}
fn tail(ui: &AppWindow) -> Result<()> {
    let expected = (ui.chat_visible_height() - ui.chat_total_height()).min(0.0);
    ensure!(
        (ui.chat_scroll_y() - expected).abs() < 3.0,
        "viewport did not reach its current bottom: y={} expected={expected}",
        ui.chat_scroll_y()
    );
    Ok(())
}
#[derive(Default)]
struct Progress {
    stage: usize,
    ticks: usize,
    anchor: f32,
    error: Option<String>,
    passed: bool,
}
fn step(ui: &AppWindow, progress: &mut Progress) -> Result<()> {
    progress.ticks += 1;
    ensure!(
        progress.ticks < 40,
        "synthetic UI layout did not settle in time"
    );
    if ui.chat_visible_height() < 100.0 {
        return Ok(());
    }
    if matches!(progress.stage, 0 | 7 | 9 | 11) {
        let expected = (ui.chat_visible_height() - ui.chat_total_height()).min(0.0);
        if (ui.chat_scroll_y() - expected).abs() >= 3.0 {
            return Ok(());
        }
    }
    match progress.stage {
        0 => {
            tail(ui)?;
            ensure!(
                ui.chat_total_height() > ui.chat_visible_height() * 2.0,
                "fixture must scroll"
            );
            ui.set_chat_scroll_y(-400.0);
            ui.prepare_scroll(3);
        }
        1 => {
            ensure!(
                (ui.chat_scroll_y() + 400.0).abs() < 3.0,
                "reader position was not selected"
            );
            ensure!(
                !ui.get_following_tail(),
                "history reader must stop following"
            );
            apply(ui, page(0, 20, true), false, 3);
        }
        2 => {
            ensure!(
                (ui.chat_scroll_y() + 400.0).abs() < 3.0,
                "unchanged poll moved the reader"
            );
            apply(ui, page(1, 21, true), false, 3);
        }
        3 => {
            ensure!(
                (ui.chat_scroll_y() + 400.0).abs() < 3.0,
                "incoming row moved a history reader"
            );
            ui.set_chat_scroll_y(-120.0);
            ui.prepare_scroll(3);
        }
        4 => {
            progress.anchor = ui.chat_scroll_y() + ui.chat_total_height();
            apply(ui, page(-10, 0, false), true, 2);
        }
        5 => {
            ensure!(
                (ui.chat_scroll_y() + ui.chat_total_height() - progress.anchor).abs() < 3.0,
                "older prepend lost its content anchor"
            );
            ensure!(!ui.get_has_more(), "older-page flag must update");
            ui.set_page("services".into());
        }
        6 => ui.set_page("hub".into()),
        7 => {
            tail(ui)?;
            ui.set_chat_scroll_y(-300.0);
            ui.prepare_scroll(3);
        }
        8 => {
            ensure!(
                (ui.chat_scroll_y() + 300.0).abs() < 3.0,
                "reader position did not settle"
            );
            ui.open_hub();
        }
        9 => {
            tail(ui)?;
            ui.set_chat_scroll_y(-300.0);
            ui.prepare_scroll(3);
        }
        10 => apply(ui, page(2, 22, true), false, 1),
        11 => {
            tail(ui)?;
            ensure!(
                ui.get_events()
                    .iter()
                    .last()
                    .is_some_and(|row| row.id == "synthetic-0021"),
                "chronological last row must be newest"
            );
            progress.passed = true;
            let _ = slint::quit_event_loop();
        }
        _ => {}
    }
    progress.stage += 1;
    Ok(())
}
fn main() -> Result<()> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    let ui = AppWindow::new()?;
    ui.set_connection_state("Prévia sintética · sem rede".into());
    ui.set_status_message("Teste de rolagem com mensagens fictícias.".into());
    ui.set_selected_day("2026-11-14".into());
    apply(&ui, page(0, 20, true), false, 1);
    let progress = Rc::new(RefCell::new(Progress::default()));
    let timer = slint::Timer::default();
    let weak = ui.as_weak();
    let state = progress.clone();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(180),
        move || {
            if let Some(ui) = weak.upgrade() {
                let outcome = {
                    let mut progress = state.borrow_mut();
                    step(&ui, &mut progress)
                };
                if let Err(error) = outcome {
                    let stage = state.borrow().stage;
                    state.borrow_mut().error = Some(format!("stage {stage}: {error}"));
                    let _ = slint::quit_event_loop();
                }
            }
        },
    );
    ui.run()?;
    timer.stop();
    let progress = progress.borrow();
    ensure!(
        progress.error.is_none(),
        "{}",
        progress.error.as_deref().unwrap_or("")
    );
    ensure!(progress.passed, "synthetic viewport checks did not finish");
    println!(
        "Synthetic desktop scroll passed: initial/open/send tail, unchanged poll, history reader, older anchor, chronological order."
    );
    Ok(())
}
