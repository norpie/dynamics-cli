use crate::tui::command::{AppId, Command};
use crate::tui::Resource;
use std::collections::HashMap;
use super::super::{Msg, FetchType, FetchedData, ExamplePair, fetch_with_cache, extract_relationships};
use super::super::app::State;
use super::super::matching::recompute_all_matches;

pub fn handle_parallel_data_loaded(
    state: &mut State,
    _task_idx: usize,
    result: Result<FetchedData, String>
) -> Command<Msg> {
    match result {
        Ok(data) => {
            // Update the appropriate metadata based on the data variant
            match data {
                FetchedData::SourceFields(fields) => {
                    if let Resource::Success(ref mut meta) = state.source_metadata {
                        meta.fields = fields;
                    } else {
                        state.source_metadata = Resource::Success(crate::api::EntityMetadata {
                            fields,
                            ..Default::default()
                        });
                    }
                }
                FetchedData::SourceForms(forms) => {
                    if let Resource::Success(ref mut meta) = state.source_metadata {
                        meta.forms = forms;
                    } else {
                        state.source_metadata = Resource::Success(crate::api::EntityMetadata {
                            forms,
                            ..Default::default()
                        });
                    }
                }
                FetchedData::SourceViews(views) => {
                    if let Resource::Success(ref mut meta) = state.source_metadata {
                        meta.views = views;
                    } else {
                        state.source_metadata = Resource::Success(crate::api::EntityMetadata {
                            views,
                            ..Default::default()
                        });
                    }
                }
                FetchedData::TargetFields(fields) => {
                    if let Resource::Success(ref mut meta) = state.target_metadata {
                        meta.fields = fields;
                    } else {
                        state.target_metadata = Resource::Success(crate::api::EntityMetadata {
                            fields,
                            ..Default::default()
                        });
                    }
                }
                FetchedData::TargetForms(forms) => {
                    if let Resource::Success(ref mut meta) = state.target_metadata {
                        meta.forms = forms;
                    } else {
                        state.target_metadata = Resource::Success(crate::api::EntityMetadata {
                            forms,
                            ..Default::default()
                        });
                    }
                }
                FetchedData::TargetViews(views) => {
                    if let Resource::Success(ref mut meta) = state.target_metadata {
                        meta.views = views;
                    } else {
                        state.target_metadata = Resource::Success(crate::api::EntityMetadata {
                            views,
                            ..Default::default()
                        });
                    }
                }
                FetchedData::ExampleData(pair_id, source_data, target_data) => {
                    // Store example data in cache
                    if let Some(pair) = state.examples.pairs.iter().find(|p| p.id == pair_id) {
                        log::info!("Fetched example data for pair {}: source_id={}, target_id={}",
                            pair_id, pair.source_record_id, pair.target_record_id);
                        state.examples.cache.insert(pair.source_record_id.clone(), source_data);
                        state.examples.cache.insert(pair.target_record_id.clone(), target_data);
                    }
                }
            }

            // Extract relationships from fields after fields are loaded
            if let Resource::Success(ref mut meta) = state.source_metadata {
                if meta.relationships.is_empty() && !meta.fields.is_empty() {
                    meta.relationships = extract_relationships(&meta.fields);
                }
            }
            if let Resource::Success(ref mut meta) = state.target_metadata {
                if meta.relationships.is_empty() && !meta.fields.is_empty() {
                    meta.relationships = extract_relationships(&meta.fields);
                }
            }

            // Write complete metadata to cache and focus tree when both fully loaded
            if let (Resource::Success(source), Resource::Success(target)) =
                (&state.source_metadata, &state.target_metadata)
            {
                if !source.fields.is_empty() && !target.fields.is_empty()
                    && !source.forms.is_empty() && !target.forms.is_empty()
                    && !source.views.is_empty() && !target.views.is_empty() {

                    // Compute all matches using the extracted function
                    let (field_matches, relationship_matches, entity_matches, source_entities, target_entities) =
                        recompute_all_matches(
                            source,
                            target,
                            &state.field_mappings,
                            &state.prefix_mappings,
                        );

                    state.field_matches = field_matches;
                    state.relationship_matches = relationship_matches;
                    state.entity_matches = entity_matches;
                    state.source_entities = source_entities;
                    state.target_entities = target_entities;

                    // Cache both metadata objects asynchronously
                    let source_env = state.source_env.clone();
                    let source_entity = state.source_entity.clone();
                    let source_meta = source.clone();
                    tokio::spawn(async move {
                        let config = crate::global_config();
                        if let Err(e) = config.set_entity_metadata_cache(&source_env, &source_entity, &source_meta).await {
                            log::error!("Failed to cache source metadata for {}/{}: {}", source_env, source_entity, e);
                        } else {
                            log::debug!("Cached source metadata for {}/{}", source_env, source_entity);
                        }
                    });

                    let target_env = state.target_env.clone();
                    let target_entity = state.target_entity.clone();
                    let target_meta = target.clone();
                    tokio::spawn(async move {
                        let config = crate::global_config();
                        if let Err(e) = config.set_entity_metadata_cache(&target_env, &target_entity, &target_meta).await {
                            log::error!("Failed to cache target metadata for {}/{}: {}", target_env, target_entity, e);
                        } else {
                            log::debug!("Cached target metadata for {}/{}", target_env, target_entity);
                        }
                    });

                    return Command::set_focus("source_tree".into());
                }
            }
        }
        Err(e) => {
            log::error!("Failed to load metadata: {}", e);
            // Navigate to error screen
            return Command::start_app(
                AppId::ErrorScreen,
                crate::tui::apps::screens::ErrorScreenParams {
                    message: format!("Failed to load entity metadata:\n\n{}", e),
                    target: Some(AppId::MigrationComparisonSelect),
                }
            );
        }
    }

    Command::None
}

pub fn handle_mappings_loaded(
    state: &mut State,
    field_mappings: HashMap<String, String>,
    prefix_mappings: HashMap<String, String>,
    example_pairs: Vec<ExamplePair>
) -> Command<Msg> {
    // Update state with loaded mappings and examples
    state.field_mappings = field_mappings;
    state.prefix_mappings = prefix_mappings;
    state.examples.pairs = example_pairs.clone();

    // Set first pair as active if any exist
    if !state.examples.pairs.is_empty() {
        state.examples.active_pair_id = Some(state.examples.pairs[0].id.clone());
    }

    // Now load metadata + example data in one parallel batch
    let mut builder = Command::perform_parallel()
        // Source entity fetches
        .add_task(
            format!("Loading {} fields ({})", state.source_entity, state.source_env),
            {
                let env = state.source_env.clone();
                let entity = state.source_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::SourceFields, true).await
                }
            }
        )
        .add_task(
            format!("Loading {} forms ({})", state.source_entity, state.source_env),
            {
                let env = state.source_env.clone();
                let entity = state.source_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::SourceForms, true).await
                }
            }
        )
        .add_task(
            format!("Loading {} views ({})", state.source_entity, state.source_env),
            {
                let env = state.source_env.clone();
                let entity = state.source_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::SourceViews, true).await
                }
            }
        )
        // Target entity fetches
        .add_task(
            format!("Loading {} fields ({})", state.target_entity, state.target_env),
            {
                let env = state.target_env.clone();
                let entity = state.target_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::TargetFields, true).await
                }
            }
        )
        .add_task(
            format!("Loading {} forms ({})", state.target_entity, state.target_env),
            {
                let env = state.target_env.clone();
                let entity = state.target_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::TargetForms, true).await
                }
            }
        )
        .add_task(
            format!("Loading {} views ({})", state.target_entity, state.target_env),
            {
                let env = state.target_env.clone();
                let entity = state.target_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::TargetViews, true).await
                }
            }
        );

    // Add example data fetching tasks
    for pair in example_pairs {
        let pair_id = pair.id.clone();
        let source_env = state.source_env.clone();
        let source_entity = state.source_entity.clone();
        let source_record_id = pair.source_record_id.clone();
        let target_env = state.target_env.clone();
        let target_entity = state.target_entity.clone();
        let target_record_id = pair.target_record_id.clone();

        builder = builder.add_task(
            format!("Loading example: {}", pair.display_name()),
            async move {
                super::super::fetch_example_pair_data(
                    &source_env,
                    &source_entity,
                    &source_record_id,
                    &target_env,
                    &target_entity,
                    &target_record_id,
                ).await
                .map(|(source, target)| FetchedData::ExampleData(pair_id, source, target))
                .map_err(|e| e.to_string())
            }
        );
    }

    builder
        .with_title("Loading Entity Comparison")
        .on_complete(AppId::EntityComparison)
        .build(|_task_idx, result| {
            let data = result.downcast::<Result<FetchedData, String>>().unwrap();
            Msg::ParallelDataLoaded(0, *data)
        })
}

pub fn handle_refresh(state: &mut State) -> Command<Msg> {
    // Re-fetch metadata for both entities
    state.source_metadata = Resource::Loading;
    state.target_metadata = Resource::Loading;

    // Clear example cache to force re-fetch
    state.examples.cache.clear();

    let mut builder = Command::perform_parallel()
        // Source entity fetches - bypass cache for manual refresh
        .add_task(
            format!("Refreshing {} fields ({})", state.source_entity, state.source_env),
            {
                let env = state.source_env.clone();
                let entity = state.source_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::SourceFields, false).await
                }
            }
        )
        .add_task(
            format!("Refreshing {} forms ({})", state.source_entity, state.source_env),
            {
                let env = state.source_env.clone();
                let entity = state.source_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::SourceForms, false).await
                }
            }
        )
        .add_task(
            format!("Refreshing {} views ({})", state.source_entity, state.source_env),
            {
                let env = state.source_env.clone();
                let entity = state.source_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::SourceViews, false).await
                }
            }
        )
        // Target entity fetches - bypass cache for manual refresh
        .add_task(
            format!("Refreshing {} fields ({})", state.target_entity, state.target_env),
            {
                let env = state.target_env.clone();
                let entity = state.target_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::TargetFields, false).await
                }
            }
        )
        .add_task(
            format!("Refreshing {} forms ({})", state.target_entity, state.target_env),
            {
                let env = state.target_env.clone();
                let entity = state.target_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::TargetForms, false).await
                }
            }
        )
        .add_task(
            format!("Refreshing {} views ({})", state.target_entity, state.target_env),
            {
                let env = state.target_env.clone();
                let entity = state.target_entity.clone();
                async move {
                    fetch_with_cache(&env, &entity, FetchType::TargetViews, false).await
                }
            }
        );

    // Add example data fetching tasks
    for pair in &state.examples.pairs {
        let pair_id = pair.id.clone();
        let source_env = state.source_env.clone();
        let source_entity = state.source_entity.clone();
        let source_record_id = pair.source_record_id.clone();
        let target_env = state.target_env.clone();
        let target_entity = state.target_entity.clone();
        let target_record_id = pair.target_record_id.clone();

        builder = builder.add_task(
            format!("Refreshing example: {}", pair.display_name()),
            async move {
                super::super::fetch_example_pair_data(
                    &source_env,
                    &source_entity,
                    &source_record_id,
                    &target_env,
                    &target_entity,
                    &target_record_id,
                ).await
                .map(|(source, target)| FetchedData::ExampleData(pair_id, source, target))
                .map_err(|e| e.to_string())
            }
        );
    }

    builder
        .with_title("Refreshing Entity Comparison")
        .on_complete(AppId::EntityComparison)
        .build(|_task_idx, result| {
            let data = result.downcast::<Result<FetchedData, String>>().unwrap();
            Msg::ParallelDataLoaded(0, *data)
        })
}
