# Conflict UI Redesign

### Context
The Folder Name Conflicts UI was too noisy, cluttered with badges, and had an ambiguous bottom action button that looked like a global queue action. 

### Changes
- Replaced the card layout with a clean accordion-style list (Opsi 3).
- Selected (Keep) folder shows minimal details in a green state.
- Unselected folders expand to show inline Rename input or Trash action.
- Removed FolderConflictActionSummary.tsx (the ambiguous global footer).
- Moved the "Resolve and Next" action button inline at the bottom of the right section.
- Cleaned up the left sidebar queue to be more subtle (removed redundant numbering, softer active states).

### Impacted Files
- src/widgets/mod-explorer/modals/FolderConflictManager.tsx (modified)
- src/widgets/mod-explorer/modals/FolderConflictCandidateCard.tsx (modified)
- src/widgets/mod-explorer/modals/FolderConflictActionSummary.tsx (removed)

### Goal
A much cleaner, focused, and intuitive conflict resolution experience where users select one winner and directly assign consequences to the rest.

### Impact
- Reduces visual fatigue during conflict resolution.
- Resolves confusion about the scope of the Resolve action.

### Notes
Decided on Opsi 3 style based on user preference for progressive disclosure of rename/trash actions for unselected candidates.
