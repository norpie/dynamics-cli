// Future migration TUI apps will go here
// See todo.md for implementation plan

pub mod migration_environment_app;
pub mod migration_comparison_select_app;
pub mod entity_comparison_app;

pub use migration_environment_app::{MigrationEnvironmentApp, State as MigrationEnvironmentState};
pub use migration_comparison_select_app::{MigrationComparisonSelectApp, State as MigrationComparisonSelectState, MigrationSelectParams};
pub use entity_comparison_app::{EntityComparisonApp, State as EntityComparisonState, EntityComparisonParams};
