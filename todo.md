# SSHS Development TODO - Issue #13 Continuation

## Current Task: Issue #13 - Enhanced Tab Features
**Status**: Phase 2 Complete - Activity Indicators Implemented
**Started**: December 11, 2025

## Progress Tracker
- [x] Issue #13 MVP: Basic tab management ✅ (deployed as v4.8.0)
- [x] Phase 1: Remove 3-session limit (practical improvement) ✅
- [x] Phase 2: Add activity indicators (*, !, @) ✅
- [ ] Phase 3: Implement session manager overlay
- [ ] Phase 4: Quality checks and deployment

## Building on v4.8.0 Foundation ✅
**Current Implementation**:
- TabManager and Session structs in `src/ui/tabs.rs`
- Basic keyboard controls (Ctrl+N, Ctrl+1/2/3)
- Visual tab bar with `▶` highlighting
- 3-session limit with proper error handling
- 41 passing tests with comprehensive coverage

## Next Enhancement Plan

### Phase 1: Remove Session Limits (TDD Implementation)
**Goal**: Increase session limit from 3 to reasonable number (20)
**Scope**:
- Update `MAX_SESSIONS` constant
- Handle tab overflow in UI rendering
- Update tests for new limits
- Add horizontal scrolling for many tabs

**Technical Tasks**:
1. Update `MAX_SESSIONS` from 3 to 20
2. Modify tab display for overflow scenarios
3. Update existing tests
4. Add new tests for many sessions
5. Implement tab overflow UI

### Phase 2: Activity Indicators
**Goal**: Add visual status indicators to tabs
**Scope**:
- `*` for new output/activity
- `!` for errors/disconnections
- `@` for background processes
- Status colors (Green/Yellow/Red)

### Phase 3: Session Manager Overlay
**Goal**: Detailed session management interface
**Scope**:
- `Ctrl+Shift+S` hotkey
- Tabular session overview
- Management commands (rename, close, etc.)

## Implementation Order (TDD)
1. Remove session limit + overflow tests
2. Add activity indicator infrastructure + tests
3. Implement session manager UI + tests
4. Integration testing and refinement

## Notes
- Following TDD methodology
- Building incrementally on proven foundation
- Maintaining backward compatibility
- Focus on practical improvements first
