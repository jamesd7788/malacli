# User Test Plan

These tests are for the product, not just the code. The goal is to validate whether the app is actually useful for reading and study.

## Session 1: First-use clarity

Goal: can a new user figure out the core loop without help?

1. Launch the app.
2. Ask the tester to find `John 3:16`.
3. Ask them to search for `light`.
4. Ask them to open a cross reference from the current verse.
5. Ask them to return to normal reading and move to the next chapter.

Watch for:
- whether `g` and `/` are discoverable
- whether the active pane is obvious
- whether opening a result feels immediate or confusing
- whether the footer text is enough guidance

Success bar:
- user completes all five tasks in under 2 minutes
- no more than one verbal hint from the moderator

## Session 2: Study workflow

Goal: validate the cross-reference interaction model.

1. Jump to `John 1:1`.
2. Open the top three cross references.
3. Ask the tester which traversal felt most useful.
4. Ask them to describe whether they felt lost or oriented.

Watch for:
- whether users understand that refs are ranked
- whether moving between source and target feels safe
- whether they need a visible history indicator soon

Success bar:
- user can follow references without losing trust in context
- user asks for deeper traversal, not a different app structure

## Session 3: Search quality

Goal: determine if linear search is already good enough for v1.

Tasks:
1. Search `beginning`.
2. Search `faith`.
3. Search `fear not`.
4. Search `charity`.

Watch for:
- whether top results feel plausible
- whether phrase-like searches are satisfying
- whether the preview length is enough to choose quickly

Success bar:
- user can identify a desired verse from the first screen of results most of the time

## Session 4: Reading quality

Goal: assess the visual and ergonomic feel.

Tasks:
1. Read one full chapter in the app.
2. Move verse by verse for thirty seconds.
3. Switch chapters back and forth.

Watch for:
- eye fatigue
- line wrapping issues
- whether the selected verse styling is too loud or too subtle
- whether the right pane steals attention from the reading pane

Success bar:
- user would choose this over opening a browser for at least one reading session

## What to log after each test

- task completion time
- mis-keys or dead ends
- points of hesitation longer than 3 seconds
- exact phrases the tester uses like `I expected...`
- whether confusion is caused by copy, layout, or missing behavior

## Immediate likely follow-ups

If these tests go well, the next product changes to validate are:

1. history/backstack
2. search result ranking improvements
3. stronger jump parser coverage
4. pinned secondary reference view
