use serde::{Deserialize, Serialize};
use turbo_tasks::{
    event::{Event, EventListener},
    util::SharedError,
    CellId, KeyValuePair, SharedReference, TaskId, ValueTypeId,
};

use crate::backend::indexed::Indexed;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellRef {
    pub task: TaskId,
    pub cell: CellId,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectiblesRef {
    pub task: TaskId,
    pub collectible_type: ValueTypeId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputValue {
    Cell(CellRef),
    Output(TaskId),
    Error,
    Panic,
}
impl OutputValue {
    fn is_transient(&self) -> bool {
        match self {
            OutputValue::Cell(cell) => cell.task.is_transient(),
            OutputValue::Output(task) => task.is_transient(),
            OutputValue::Error => false,
            OutputValue::Panic => false,
        }
    }
}

#[derive(Debug)]
pub struct RootState {
    pub ty: ActiveType,
    pub all_clean_event: Event,
}

impl RootState {
    pub fn new(ty: ActiveType) -> Self {
        Self {
            ty,
            all_clean_event: Event::new(|| "RootState::all_clean_event".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ActiveType {
    RootTask,
    OnceTask,
    /// The aggregated task graph was scheduled because it has reached an AggregatedRoot while
    /// propagating the dirty container or is read strongly consistent. This state is reset when
    /// all this sub graph becomes clean again.
    CachedActiveUntilClean,
}

impl Clone for RootState {
    fn clone(&self) -> Self {
        panic!("RootState cannot be cloned");
    }
}

#[derive(Debug)]
pub enum InProgressState {
    Scheduled {
        done_event: Event,
    },
    InProgress {
        stale: bool,
        #[allow(dead_code)]
        once_task: bool,
        done_event: Event,
    },
}

impl Clone for InProgressState {
    fn clone(&self) -> Self {
        panic!("InProgressState cannot be cloned");
    }
}

#[derive(Debug)]
pub struct InProgressCellState {
    pub event: Event,
}

impl Clone for InProgressCellState {
    fn clone(&self) -> Self {
        panic!("InProgressCell cannot be cloned");
    }
}

impl InProgressCellState {
    pub fn new(task_id: TaskId, cell: CellId) -> Self {
        InProgressCellState {
            event: Event::new(move || {
                format!("InProgressCellState::event ({} {:?})", task_id, cell)
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AggregationNumber {
    pub base: u32,
    pub distance: u32,
    pub effective: u32,
}

#[derive(Debug, Clone, KeyValuePair)]
pub enum CachedDataItem {
    // Output
    Output {
        value: OutputValue,
    },
    Collectible {
        collectible: CellRef,
        value: (),
    },

    // State
    Dirty {
        value: (),
    },
    DirtyWhenPersisted {
        value: (),
    },

    // Children
    Child {
        task: TaskId,
        value: (),
    },

    // Cells
    CellData {
        cell: CellId,
        value: SharedReference,
    },
    CellTypeMaxIndex {
        cell_type: ValueTypeId,
        value: u32,
    },

    // Dependencies
    OutputDependency {
        target: TaskId,
        value: (),
    },
    CellDependency {
        target: CellRef,
        value: (),
    },
    CollectiblesDependency {
        target: CollectiblesRef,
        value: (),
    },

    // Dependent
    OutputDependent {
        task: TaskId,
        value: (),
    },
    CellDependent {
        cell: CellId,
        task: TaskId,
        value: (),
    },
    CollectiblesDependent {
        collectibles_type: ValueTypeId,
        task: TaskId,
        value: (),
    },

    // Aggregation Graph
    AggregationNumber {
        value: AggregationNumber,
    },
    Follower {
        task: TaskId,
        value: i32,
    },
    Upper {
        task: TaskId,
        value: i32,
    },

    // Aggregated Data
    AggregatedDirtyContainer {
        task: TaskId,
        value: i32,
    },
    AggregatedCollectible {
        collectible: CellRef,
        value: i32,
    },
    AggregatedDirtyContainerCount {
        value: i32,
    },

    // Transient Root Type
    AggregateRoot {
        value: RootState,
    },

    // Transient In Progress state
    InProgress {
        value: InProgressState,
    },
    InProgressCell {
        cell: CellId,
        value: InProgressCellState,
    },
    OutdatedCollectible {
        collectible: CellRef,
        value: (),
    },
    OutdatedOutputDependency {
        target: TaskId,
        value: (),
    },
    OutdatedCellDependency {
        target: CellRef,
        value: (),
    },
    OutdatedChild {
        task: TaskId,
        value: (),
    },

    // Transient Error State
    Error {
        value: SharedError,
    },
}

impl CachedDataItem {
    pub fn is_persistent(&self) -> bool {
        match self {
            CachedDataItem::Output { value } => value.is_transient(),
            CachedDataItem::Collectible { collectible, .. } => !collectible.task.is_transient(),
            CachedDataItem::Dirty { .. } => true,
            CachedDataItem::DirtyWhenPersisted { .. } => true,
            CachedDataItem::Child { task, .. } => !task.is_transient(),
            CachedDataItem::CellData { .. } => true,
            CachedDataItem::CellTypeMaxIndex { .. } => true,
            CachedDataItem::OutputDependency { target, .. } => !target.is_transient(),
            CachedDataItem::CellDependency { target, .. } => !target.task.is_transient(),
            CachedDataItem::CollectiblesDependency { target, .. } => !target.task.is_transient(),
            CachedDataItem::OutputDependent { task, .. } => !task.is_transient(),
            CachedDataItem::CellDependent { task, .. } => !task.is_transient(),
            CachedDataItem::CollectiblesDependent { task, .. } => !task.is_transient(),
            CachedDataItem::AggregationNumber { .. } => true,
            CachedDataItem::Follower { task, .. } => !task.is_transient(),
            CachedDataItem::Upper { task, .. } => !task.is_transient(),
            CachedDataItem::AggregatedDirtyContainer { task, .. } => !task.is_transient(),
            CachedDataItem::AggregatedCollectible { collectible, .. } => {
                !collectible.task.is_transient()
            }
            CachedDataItem::AggregatedDirtyContainerCount { .. } => true,
            CachedDataItem::AggregateRoot { .. } => false,
            CachedDataItem::InProgress { .. } => false,
            CachedDataItem::InProgressCell { .. } => false,
            CachedDataItem::OutdatedCollectible { .. } => false,
            CachedDataItem::OutdatedOutputDependency { .. } => false,
            CachedDataItem::OutdatedCellDependency { .. } => false,
            CachedDataItem::OutdatedChild { .. } => false,
            CachedDataItem::Error { .. } => false,
        }
    }

    pub fn new_scheduled(description: impl Fn() -> String + Sync + Send + 'static) -> Self {
        CachedDataItem::InProgress {
            value: InProgressState::Scheduled {
                done_event: Event::new(move || format!("{} done_event", description())),
            },
        }
    }

    pub fn new_scheduled_with_listener(
        description: impl Fn() -> String + Sync + Send + 'static,
        note: impl Fn() -> String + Sync + Send + 'static,
    ) -> (Self, EventListener) {
        let done_event = Event::new(move || format!("{} done_event", description()));
        let listener = done_event.listen_with_note(note);
        (
            CachedDataItem::InProgress {
                value: InProgressState::Scheduled { done_event },
            },
            listener,
        )
    }
}

impl CachedDataItemKey {
    pub fn is_persistent(&self) -> bool {
        match self {
            CachedDataItemKey::Output { .. } => true,
            CachedDataItemKey::Collectible { collectible, .. } => !collectible.task.is_transient(),
            CachedDataItemKey::Dirty { .. } => true,
            CachedDataItemKey::DirtyWhenPersisted { .. } => true,
            CachedDataItemKey::Child { task, .. } => !task.is_transient(),
            CachedDataItemKey::CellData { .. } => true,
            CachedDataItemKey::CellTypeMaxIndex { .. } => true,
            CachedDataItemKey::OutputDependency { target, .. } => !target.is_transient(),
            CachedDataItemKey::CellDependency { target, .. } => !target.task.is_transient(),
            CachedDataItemKey::CollectiblesDependency { target, .. } => !target.task.is_transient(),
            CachedDataItemKey::OutputDependent { task, .. } => !task.is_transient(),
            CachedDataItemKey::CellDependent { task, .. } => !task.is_transient(),
            CachedDataItemKey::CollectiblesDependent { task, .. } => !task.is_transient(),
            CachedDataItemKey::AggregationNumber { .. } => true,
            CachedDataItemKey::Follower { task, .. } => !task.is_transient(),
            CachedDataItemKey::Upper { task, .. } => !task.is_transient(),
            CachedDataItemKey::AggregatedDirtyContainer { task, .. } => !task.is_transient(),
            CachedDataItemKey::AggregatedCollectible { collectible, .. } => {
                !collectible.task.is_transient()
            }
            CachedDataItemKey::AggregatedDirtyContainerCount { .. } => true,
            CachedDataItemKey::AggregateRoot { .. } => false,
            CachedDataItemKey::InProgress { .. } => false,
            CachedDataItemKey::InProgressCell { .. } => false,
            CachedDataItemKey::OutdatedCollectible { .. } => false,
            CachedDataItemKey::OutdatedOutputDependency { .. } => false,
            CachedDataItemKey::OutdatedCellDependency { .. } => false,
            CachedDataItemKey::OutdatedChild { .. } => false,
            CachedDataItemKey::Error { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CachedDataItemIndex {
    Children,
    Follower,
    Upper,
    AggregatedDirtyContainer,
    CellData,
    CellTypeMaxIndex,
    CellDependent,
    OutputDependent,
    Dependencies,
}

#[allow(non_upper_case_globals, dead_code)]
pub mod indicies {
    use super::CachedDataItemIndex;

    pub const Child: CachedDataItemIndex = CachedDataItemIndex::Children;
    pub const OutdatedChild: CachedDataItemIndex = CachedDataItemIndex::Children;
    pub const Follower: CachedDataItemIndex = CachedDataItemIndex::Follower;
    pub const Upper: CachedDataItemIndex = CachedDataItemIndex::Upper;
    pub const AggregatedDirtyContainer: CachedDataItemIndex =
        CachedDataItemIndex::AggregatedDirtyContainer;
    pub const CellData: CachedDataItemIndex = CachedDataItemIndex::CellData;
    pub const CellTypeMaxIndex: CachedDataItemIndex = CachedDataItemIndex::CellTypeMaxIndex;
    pub const CellDependent: CachedDataItemIndex = CachedDataItemIndex::CellDependent;
    pub const OutputDependent: CachedDataItemIndex = CachedDataItemIndex::OutputDependent;
    pub const OutputDependency: CachedDataItemIndex = CachedDataItemIndex::Dependencies;
    pub const CellDependency: CachedDataItemIndex = CachedDataItemIndex::Dependencies;
    pub const OutdatedOutputDependency: CachedDataItemIndex = CachedDataItemIndex::Dependencies;
    pub const OutdatedCellDependency: CachedDataItemIndex = CachedDataItemIndex::Dependencies;
}

impl Indexed for CachedDataItemKey {
    type Index = Option<CachedDataItemIndex>;

    fn index(&self) -> Option<CachedDataItemIndex> {
        match self {
            CachedDataItemKey::Child { .. } => Some(CachedDataItemIndex::Children),
            CachedDataItemKey::OutdatedChild { .. } => Some(CachedDataItemIndex::Children),
            CachedDataItemKey::Follower { .. } => Some(CachedDataItemIndex::Follower),
            CachedDataItemKey::Upper { .. } => Some(CachedDataItemIndex::Upper),
            CachedDataItemKey::AggregatedDirtyContainer { .. } => {
                Some(CachedDataItemIndex::AggregatedDirtyContainer)
            }
            CachedDataItemKey::CellData { .. } => Some(CachedDataItemIndex::CellData),
            CachedDataItemKey::CellTypeMaxIndex { .. } => {
                Some(CachedDataItemIndex::CellTypeMaxIndex)
            }
            CachedDataItemKey::CellDependent { .. } => Some(CachedDataItemIndex::CellDependent),
            CachedDataItemKey::OutputDependent { .. } => Some(CachedDataItemIndex::OutputDependent),
            CachedDataItemKey::OutputDependency { .. } => Some(CachedDataItemIndex::Dependencies),
            CachedDataItemKey::CellDependency { .. } => Some(CachedDataItemIndex::Dependencies),
            CachedDataItemKey::OutdatedOutputDependency { .. } => {
                Some(CachedDataItemIndex::Dependencies)
            }
            CachedDataItemKey::OutdatedCellDependency { .. } => {
                Some(CachedDataItemIndex::Dependencies)
            }
            _ => None,
        }
    }
}

impl CachedDataItemValue {
    pub fn is_persistent(&self) -> bool {
        match self {
            CachedDataItemValue::Output { value } => !value.is_transient(),
            _ => true,
        }
    }
}

#[derive(Debug)]
pub struct CachedDataUpdate {
    // TODO persistence
    #[allow(dead_code)]
    pub task: TaskId,
    #[allow(dead_code)]
    pub key: CachedDataItemKey,
    #[allow(dead_code)]
    pub value: Option<CachedDataItemValue>,
}
