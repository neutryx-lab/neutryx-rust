//! Static dispatch enum for workspace implementations.

use enum_dispatch::enum_dispatch;

use super::{
    layout_config::PathLayout, workspace::PathWorkspace,
    workspace_timestep_first::TimeStepFirstWorkspace,
};

/// Static dispatch enum for workspace implementations.
#[enum_dispatch(PathWorkspaceTrait)]
pub enum WorkspaceEnum {
    /// Traditional path-first layout: `[path][step]`
    PathFirst(PathWorkspace),
    /// Optimised time-step-first layout: `[step][path]`
    TimeStepFirst(TimeStepFirstWorkspace),
}

impl WorkspaceEnum {
    /// Creates a workspace with the specified layout.
    pub fn new(layout: PathLayout, num_paths: usize, num_steps: usize) -> Self {
        match layout {
            PathLayout::PathFirst => Self::PathFirst(PathWorkspace::new(num_paths, num_steps)),
            PathLayout::TimeStepFirst => {
                Self::TimeStepFirst(TimeStepFirstWorkspace::new(num_paths, num_steps))
            }
        }
    }

    /// Creates a PathFirst workspace.
    #[inline]
    pub fn path_first(num_paths: usize, num_steps: usize) -> Self {
        Self::PathFirst(PathWorkspace::new(num_paths, num_steps))
    }

    /// Creates a TimeStepFirst workspace.
    #[inline]
    pub fn timestep_first(num_paths: usize, num_steps: usize) -> Self {
        Self::TimeStepFirst(TimeStepFirstWorkspace::new(num_paths, num_steps))
    }

    /// Creates a TimeStepFirst workspace with custom alignment.
    #[inline]
    pub fn timestep_first_aligned(num_paths: usize, num_steps: usize, alignment: usize) -> Self {
        Self::TimeStepFirst(TimeStepFirstWorkspace::with_alignment(
            num_paths, num_steps, alignment,
        ))
    }

    /// Returns a reference to the inner PathWorkspace if PathFirst.
    #[inline]
    pub fn as_path_first(&self) -> Option<&PathWorkspace> {
        match self {
            Self::PathFirst(ws) => Some(ws),
            Self::TimeStepFirst(_) => None,
        }
    }

    /// Returns a mutable reference to the inner PathWorkspace if PathFirst.
    #[inline]
    pub fn as_path_first_mut(&mut self) -> Option<&mut PathWorkspace> {
        match self {
            Self::PathFirst(ws) => Some(ws),
            Self::TimeStepFirst(_) => None,
        }
    }

    /// Returns a reference to the inner TimeStepFirstWorkspace if
    #[inline]
    pub fn as_timestep_first(&self) -> Option<&TimeStepFirstWorkspace> {
        match self {
            Self::PathFirst(_) => None,
            Self::TimeStepFirst(ws) => Some(ws),
        }
    }

    /// Returns a mutable reference to the inner TimeStepFirstWorkspace if
    #[inline]
    pub fn as_timestep_first_mut(&mut self) -> Option<&mut TimeStepFirstWorkspace> {
        match self {
            Self::PathFirst(_) => None,
            Self::TimeStepFirst(ws) => Some(ws),
        }
    }

    /// Ensures the workspace has sufficient capacity.
    pub fn ensure_capacity(&mut self, num_paths: usize, num_steps: usize) {
        match self {
            Self::PathFirst(ws) => ws.ensure_capacity(num_paths, num_steps),
            Self::TimeStepFirst(ws) => ws.ensure_capacity(num_paths, num_steps),
        }
    }

    /// Resets the workspace for reuse.
    pub fn reset(&mut self) {
        match self {
            Self::PathFirst(ws) => ws.reset(),
            Self::TimeStepFirst(ws) => ws.reset(),
        }
    }

    /// Fast reset that preserves capacity.
    #[inline]
    pub fn reset_fast(&mut self) {
        match self {
            Self::PathFirst(ws) => ws.reset_fast(),
            Self::TimeStepFirst(ws) => ws.reset_fast(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::mc::workspace_trait::PathWorkspaceTrait;

    #[test]
    fn test_workspace_enum_new_path_first() {
        let ws = WorkspaceEnum::new(PathLayout::PathFirst, 100, 10);
        assert_eq!(ws.num_paths(), 100);
        assert_eq!(ws.num_steps(), 10);
        assert_eq!(ws.layout(), PathLayout::PathFirst);
        assert!(ws.as_path_first().is_some());
        assert!(ws.as_timestep_first().is_none());
    }

    #[test]
    fn test_workspace_enum_new_timestep_first() {
        let ws = WorkspaceEnum::new(PathLayout::TimeStepFirst, 100, 10);
        assert_eq!(ws.num_paths(), 100);
        assert_eq!(ws.num_steps(), 10);
        assert_eq!(ws.layout(), PathLayout::TimeStepFirst);
        assert!(ws.as_path_first().is_none());
        assert!(ws.as_timestep_first().is_some());
    }

    #[test]
    fn test_workspace_enum_path_first_constructor() {
        let ws = WorkspaceEnum::path_first(100, 10);
        assert_eq!(ws.layout(), PathLayout::PathFirst);
    }

    #[test]
    fn test_workspace_enum_timestep_first_constructor() {
        let ws = WorkspaceEnum::timestep_first(100, 10);
        assert_eq!(ws.layout(), PathLayout::TimeStepFirst);
    }

    #[test]
    fn test_workspace_enum_timestep_first_aligned() {
        let ws = WorkspaceEnum::timestep_first_aligned(100, 10, 128);
        assert_eq!(ws.layout(), PathLayout::TimeStepFirst);
        if let Some(inner) = ws.as_timestep_first() {
            assert_eq!(inner.alignment(), 128);
        }
    }

    #[test]
    fn test_workspace_enum_get_set_value_path_first() {
        let mut ws = WorkspaceEnum::path_first(10, 5);

        ws.set_path_value(0, 0, 100.0);
        ws.set_path_value(9, 5, 200.0);

        assert_eq!(ws.get_path_value(0, 0), 100.0);
        assert_eq!(ws.get_path_value(9, 5), 200.0);
    }

    #[test]
    fn test_workspace_enum_get_set_value_timestep_first() {
        let mut ws = WorkspaceEnum::timestep_first(10, 5);

        ws.set_path_value(0, 0, 100.0);
        ws.set_path_value(9, 5, 200.0);

        assert_eq!(ws.get_path_value(0, 0), 100.0);
        assert_eq!(ws.get_path_value(9, 5), 200.0);
    }

    #[test]
    fn test_workspace_enum_step_slice_path_first() {
        let ws = WorkspaceEnum::path_first(10, 5);
        assert!(ws.get_step_slice(0).is_none());
    }

    #[test]
    fn test_workspace_enum_step_slice_timestep_first() {
        let mut ws = WorkspaceEnum::timestep_first(10, 5);

        if let Some(step0) = ws.get_step_slice_mut(0) {
            for (i, val) in step0.iter_mut().enumerate() {
                *val = i as f64;
            }
        }

        let step0 = ws.get_step_slice(0).unwrap();
        assert_eq!(step0.len(), 10);
        assert_eq!(step0[0], 0.0);
        assert_eq!(step0[9], 9.0);
    }

    #[test]
    fn test_workspace_enum_path_slice_path_first() {
        let mut ws = WorkspaceEnum::path_first(10, 5);

        ws.set_path_value(0, 0, 100.0);
        ws.set_path_value(0, 5, 150.0);

        let path0 = ws.get_path_slice(0).unwrap();
        assert_eq!(path0.len(), 6);
        assert_eq!(path0[0], 100.0);
        assert_eq!(path0[5], 150.0);
    }

    #[test]
    fn test_workspace_enum_path_slice_timestep_first() {
        let ws = WorkspaceEnum::timestep_first(10, 5);
        assert!(ws.get_path_slice(0).is_none());
    }

    #[test]
    fn test_workspace_enum_clear() {
        let mut ws = WorkspaceEnum::timestep_first(10, 5);

        ws.set_path_value(0, 0, 100.0);
        ws.clear();

        assert_eq!(ws.get_path_value(0, 0), 0.0);
    }

    #[test]
    fn test_workspace_enum_memory_usage() {
        let ws_pf = WorkspaceEnum::path_first(100, 10);
        let ws_tsf = WorkspaceEnum::timestep_first(100, 10);

        let mem_pf = ws_pf.memory_usage();
        let mem_tsf = ws_tsf.memory_usage();

        assert!(mem_pf > 0);
        assert!(mem_tsf > 0);
    }

    #[test]
    fn test_workspace_enum_randoms_and_payoffs() {
        let mut ws = WorkspaceEnum::timestep_first(10, 5);

        ws.randoms_mut()[0] = 1.0;
        ws.payoffs_mut()[0] = 42.0;

        assert_eq!(ws.randoms()[0], 1.0);
        assert_eq!(ws.payoffs()[0], 42.0);
    }

    #[test]
    fn test_workspace_enum_ensure_capacity() {
        let mut ws = WorkspaceEnum::timestep_first(10, 5);

        ws.ensure_capacity(20, 10);

        assert_eq!(ws.num_paths(), 20);
        assert_eq!(ws.num_steps(), 10);
    }

    #[test]
    fn test_workspace_enum_reset() {
        let mut ws = WorkspaceEnum::path_first(10, 5);
        ws.set_path_value(0, 0, 100.0);

        ws.reset();

    }

    #[test]
    fn test_workspace_enum_consistent_values_across_layouts() {
        let mut ws_pf = WorkspaceEnum::path_first(4, 3);
        let mut ws_tsf = WorkspaceEnum::timestep_first(4, 3);

        for path in 0..4 {
            for step in 0..4 {
                let val = (path * 10 + step) as f64;
                ws_pf.set_path_value(path, step, val);
                ws_tsf.set_path_value(path, step, val);
            }
        }

        for path in 0..4 {
            for step in 0..4 {
                let expected = (path * 10 + step) as f64;
                assert_eq!(
                    ws_pf.get_path_value(path, step),
                    expected,
                    "PathFirst mismatch at ({}, {})",
                    path,
                    step
                );
                assert_eq!(
                    ws_tsf.get_path_value(path, step),
                    expected,
                    "TimeStepFirst mismatch at ({}, {})",
                    path,
                    step
                );
            }
        }
    }
}
