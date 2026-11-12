# Demo science bank (made up, for testing only)

These files are NOT real GED content and never ship. They add a small
"science" subject with two skills, a short week path, two shared stimuli and
one question of every format (multiple choice, number and word fill-in,
drop-down, order, sort), so the app's multi-subject features can be tested
before real Science content exists.

Build a test bank (the real content plus this demo) with:

    cargo run -q -p ged-content -- demo <OUT_DIR>

then run the server with `GED_CONTENT=<OUT_DIR>` and `GED_SERVE_DRAFTS=1`.
Files whose name already exists in content/ are appended to it; others are
copied.
