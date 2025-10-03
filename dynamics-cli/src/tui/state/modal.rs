/// Modal state pattern (similar to Resource<T>)
///
/// Represents whether a modal is open or closed, with optional data
/// when open.
///
/// # Examples
///
/// ```rust
/// // Simple confirmation modal
/// delete_modal: ModalState<()>,
///
/// // Modal with data (e.g., which item to delete)
/// delete_modal: ModalState<MigrationId>,
///
/// // Modal with form data
/// rename_modal: ModalState<RenameForm>,
/// ```
#[derive(Clone, Debug)]
pub enum ModalState<T> {
    /// Modal is closed
    Closed,

    /// Modal is open with optional data
    Open(T),
}

impl<T> Default for ModalState<T> {
    fn default() -> Self {
        ModalState::Closed
    }
}

impl<T> ModalState<T> {
    /// Create a closed modal
    pub fn closed() -> Self {
        ModalState::Closed
    }

    /// Create an open modal with data
    pub fn open(data: T) -> Self {
        ModalState::Open(data)
    }

    /// Check if the modal is open
    pub fn is_open(&self) -> bool {
        matches!(self, ModalState::Open(_))
    }

    /// Check if the modal is closed
    pub fn is_closed(&self) -> bool {
        matches!(self, ModalState::Closed)
    }

    /// Get the data if open, None otherwise
    pub fn data(&self) -> Option<&T> {
        match self {
            ModalState::Open(data) => Some(data),
            ModalState::Closed => None,
        }
    }

    /// Get mutable data if open, None otherwise
    pub fn data_mut(&mut self) -> Option<&mut T> {
        match self {
            ModalState::Open(data) => Some(data),
            ModalState::Closed => None,
        }
    }

    /// Map the data if open, keeping the modal closed if already closed
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> ModalState<U> {
        match self {
            ModalState::Open(data) => ModalState::Open(f(data)),
            ModalState::Closed => ModalState::Closed,
        }
    }

    /// Convert to Option<T>
    pub fn into_option(self) -> Option<T> {
        match self {
            ModalState::Open(data) => Some(data),
            ModalState::Closed => None,
        }
    }

    /// Convenience method to close the modal
    pub fn close(&mut self) {
        *self = ModalState::Closed;
    }

    /// Convenience method to open the modal with data
    pub fn open_with(&mut self, data: T) {
        *self = ModalState::Open(data);
    }
}

impl<T> From<Option<T>> for ModalState<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(data) => ModalState::Open(data),
            None => ModalState::Closed,
        }
    }
}

impl<T> From<ModalState<T>> for Option<T> {
    fn from(modal: ModalState<T>) -> Self {
        modal.into_option()
    }
}

/// Unit type for modals without data
impl ModalState<()> {
    /// Open a modal without data
    pub fn open_empty(&mut self) {
        *self = ModalState::Open(());
    }
}
