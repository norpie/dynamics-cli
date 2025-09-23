use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{Frame, layout::Rect};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use crate::{
    commands::migration::services::{ComparisonData, fetch_comparison_data},
    commands::migration::ui::{
        components::{
            FooterAction,
            fetch_progress::FetchProgress,
            loading_modal::{LoadingModalComponent, LoadingState},
        },
        screens::{Screen, ScreenResult, UnifiedCompareScreen, comparison::data_models::ExamplePair},
    },
    config::{Config, SavedComparison, SavedMigration},
};
use serde_json::Value;
use std::collections::HashMap;

pub struct LoadingScreen {
    config: Config,
    migration: SavedMigration,
    comparison: SavedComparison,
    modal: LoadingModalComponent,
    progress: Arc<Mutex<FetchProgress>>,
    state: LoadingScreenState,
    fetch_task: Option<tokio::task::JoinHandle<()>>,
    result_receiver: Option<oneshot::Receiver<Result<(ComparisonData, HashMap<String, Value>)>>>,
}

#[derive(Debug, Clone)]
enum LoadingScreenState {
    Loading,
    Failed(Vec<String>),
    Success(ComparisonData, HashMap<String, Value>),
}

impl LoadingScreen {
    pub fn new(config: Config, migration: SavedMigration, comparison: SavedComparison) -> Self {
        log::info!("=== LoadingScreen::new called ===");
        log::debug!("  source_entity: {}", comparison.source_entity);
        log::debug!("  target_entity: {}", comparison.target_entity);

        let progress = Arc::new(Mutex::new(FetchProgress::new()));
        let message = if comparison.source_entity == comparison.target_entity {
            format!(
                "Fetching entity metadata, views, and forms for '{}' from both environments...",
                comparison.source_entity
            )
        } else {
            format!(
                "Fetching metadata for '{}' from source and '{}' from target...",
                comparison.source_entity, comparison.target_entity
            )
        };

        let modal = LoadingModalComponent::new(message, progress.clone());

        Self {
            config,
            migration,
            comparison,
            modal,
            progress,
            state: LoadingScreenState::Loading,
            fetch_task: None,
            result_receiver: None,
        }
    }

    fn start_fetch(&mut self) -> Result<()> {
        log::info!("=== LoadingScreen::start_fetch called ===");

        let source_auth = self
            .config
            .environments
            .get(&self.migration.source_env)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Source environment '{}' not found",
                    self.migration.source_env
                )
            })?
            .clone();

        let target_auth = self
            .config
            .environments
            .get(&self.migration.target_env)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Target environment '{}' not found",
                    self.migration.target_env
                )
            })?
            .clone();

        let migration = self.migration.clone();
        let comparison = self.comparison.clone();
        let progress = self.progress.clone();

        // Load examples from config
        let examples: Vec<ExamplePair> = self.config
            .get_examples(&comparison.source_entity, &comparison.target_entity)
            .map(|config_examples| {
                log::debug!("Loaded {} examples from config for {} -> {}",
                    config_examples.len(),
                    comparison.source_entity,
                    comparison.target_entity
                );
                let examples: Vec<ExamplePair> = config_examples.iter().map(ExamplePair::from_config).collect();
                for example in &examples {
                    log::debug!("  Example: {} -> {}", example.source_uuid, example.target_uuid);
                }
                examples
            })
            .unwrap_or_else(|| {
                log::debug!("No examples found in config for {} -> {}",
                    comparison.source_entity,
                    comparison.target_entity
                );
                Vec::new()
            });

        let (sender, receiver) = oneshot::channel();
        self.result_receiver = Some(receiver);

        let task = tokio::spawn(async move {
            let result =
                fetch_comparison_data(source_auth, target_auth, migration, comparison, examples, progress)
                    .await;
            let _ = sender.send(result); // Ignore if receiver is dropped
        });

        self.fetch_task = Some(task);
        Ok(())
    }

    fn check_fetch_status(&mut self) -> Result<()> {
        if let Some(receiver) = &mut self.result_receiver {
            // Check if we have a result without blocking
            match receiver.try_recv() {
                Ok(result) => {
                    // We got a result - immediately transition without showing final render
                    match result {
                        Ok((comparison_data, example_data)) => {
                            self.state = LoadingScreenState::Success(comparison_data, example_data);
                            // No need to skip render, we'll handle navigation immediately
                        }
                        Err(_) => {
                            let progress = self.progress.lock().unwrap();
                            let error_list = progress.get_error_messages();
                            drop(progress);

                            let errors = if error_list.is_empty() {
                                vec!["Unknown error occurred during fetch".to_string()]
                            } else {
                                error_list
                            };

                            self.state = LoadingScreenState::Failed(errors.clone());
                            self.modal.set_state(LoadingState::Failed(errors));
                        }
                    }
                    self.result_receiver = None;
                    self.fetch_task = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    // Not ready yet, continue waiting
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    // Sender was dropped, task failed
                    let errors = vec!["Task was cancelled".to_string()];
                    self.state = LoadingScreenState::Failed(errors.clone());
                    self.modal.set_state(LoadingState::Failed(errors));
                    self.result_receiver = None;
                    self.fetch_task = None;
                }
            }
        }
        Ok(())
    }
}

impl Screen for LoadingScreen {
    fn on_enter(&mut self) {
        log::info!("=== LoadingScreen::on_enter called ===");
        if let Err(e) = self.start_fetch() {
            let error_msg = format!("Failed to start fetch: {}", e);
            self.state = LoadingScreenState::Failed(vec![error_msg.clone()]);
            self.modal.set_state(LoadingState::Failed(vec![error_msg]));
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        if let Err(e) = self.check_fetch_status() {
            let error_msg = format!("Error checking fetch status: {}", e);
            self.state = LoadingScreenState::Failed(vec![error_msg.clone()]);
            self.modal.set_state(LoadingState::Failed(vec![error_msg]));
        }

        // Only render if not in success state (about to navigate)
        if !matches!(self.state, LoadingScreenState::Success(_, _)) {
            self.modal.update();
            self.modal.render(f, area);
        }
    }

    fn handle_event(&mut self, event: Event) -> ScreenResult {
        match &self.state {
            LoadingScreenState::Success(_, _) => {
                // Navigation is handled by check_navigation(), but this shouldn't be reached
                ScreenResult::Continue
            }
            LoadingScreenState::Loading => {
                // No input during loading
                ScreenResult::Continue
            }
            LoadingScreenState::Failed(_) => {
                match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        match key.code {
                            KeyCode::Esc => {
                                // Go back to comparison select screen
                                ScreenResult::Back
                            }
                            _ => ScreenResult::Continue,
                        }
                    }
                    _ => ScreenResult::Continue,
                }
            }
        }
    }

    fn get_footer_actions(&self) -> Vec<FooterAction> {
        match &self.state {
            LoadingScreenState::Loading => {
                vec![] // No actions during loading
            }
            LoadingScreenState::Failed(_) => {
                vec![FooterAction {
                    key: "Esc".to_string(),
                    description: "Back".to_string(),
                    enabled: true,
                }]
            }
            LoadingScreenState::Success(_, _) => {
                vec![] // Will auto-navigate
            }
        }
    }

    fn get_title(&self) -> Option<String> {
        Some(format!("Loading - {}", self.comparison.source_entity))
    }

    fn on_exit(&mut self) {
        // Cancel the fetch task if still running
        if let Some(task) = self.fetch_task.take() {
            task.abort();
        }
        // Drop the receiver
        self.result_receiver = None;
    }

    fn check_navigation(&mut self) -> Option<ScreenResult> {
        // Immediate navigation when fetch is complete
        if let LoadingScreenState::Success(data, example_data) = &self.state {
            Some(ScreenResult::Navigate(Box::new(
                UnifiedCompareScreen::new_with_data(
                    self.config.clone(),
                    self.comparison.clone(),
                    data.source_fields.clone(),
                    data.target_fields.clone(),
                    data.source_views.clone(),
                    data.target_views.clone(),
                    data.source_forms.clone(),
                    data.target_forms.clone(),
                    data.source_env.clone(),
                    data.target_env.clone(),
                    example_data.clone(),
                ),
            )))
        } else {
            None
        }
    }
}
