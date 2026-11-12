# Writing Science, Social Studies and Language Arts content

Math answers are checked by code. Most answers in the other three tests
can't be: they depend on reading a passage, knowing a fact, or judging
which choice is best. These rules, and a stricter review, keep that content
honest. Explanations follow `docs/EXPLANATION_STYLE.md` as in Math.

Before writing for a subject, read its research notes in
`docs/subjects/<subject>.md`: the test's structure, content weights, the
official practice and content-topic codes, and the question types it uses.

## Where things go

| What | File |
| --- | --- |
| Skills (one per group of official practices) | `content/skills.toml`, with `subject = "science"` and so on |
| Principles (one per official practice code, e.g. `SP.3.b`) | `content/principles.toml` |
| Questions | `content/<subject>/<skill>.toml` |
| Shared passages, tables and figures | `content/stimuli/<subject>.toml` |
| The week path | `content/journey.toml`, stages with `subject = "…"` and `day = 1–7` |
| Study links | `content/study.toml` |

Every question has a `topic`: the official content-topic code, e.g.
`L.c.2` or `CG.b.5`. Its first part picks the domain, so practice tests can
match each test's weights (`content/subjects.toml`).

## Sources

- **Every non-math question lists at least one source** in `sources`. Give
  something a fact-checker can open: a URL with the publisher, a book
  section, or a founding document with its article and section.
- A question on one of our own passages cites the passage. Add a source for
  any outside fact the question, answer or explanation relies on.
- **Passages (stimuli) are never copyrighted text.** The `source` must start
  with one of:
  - `Original`: written by us;
  - `Public domain: <work, date>`: e.g. the Bill of Rights, a 19th-century
    speech, a US government document;
  - `Data: <source>`: a table or graph built from public data, such as NOAA
    or the Census Bureau, with numbers we may simplify.

## Passages

- Each subject sets a target reading grade in `subjects.toml` (Science and
  Social Studies about grade 10, Language Arts up to 12). Above it,
  `cargo content` warns, and the reviewer must say whether the difficulty
  is on purpose.
- Use realistic lengths for the test: short texts and graphics for Science
  and Social Studies, 400 to 900 words for Language Arts reading passages.
- Two or three questions per Science or Social Studies passage, like the
  real test; six to eight for a Language Arts passage. A passage may carry
  a few more for practice: a practice test takes at most `set_max` of them
  (`subjects.toml`; default 3, Language Arts 8), picked at random.

## Questions with one right answer

- **Multiple choice and drop-down:** every wrong choice must be clearly
  wrong to someone who understands the passage ("defensibly wrong"), yet
  tempting to someone who doesn't. Base each one on a real mistake.
- **Order:** only use it when there is exactly one correct order, such as
  steps of a process or events with dates. If two orders are defensible,
  rewrite it.
- **Sort:** each item belongs in exactly one group. Avoid items that
  "partly" fit.
- **Word fill-in:** list every reasonable wording in `accept`.

## Review: two blind solvers and a fact-checker

A non-math question is approved only when three different reviewers have
passed its current version:

1. **Two solvers.** Each is a fresh subagent that did not write it. Each
   solves it blind from `cargo content-quiz <skill>` and records the answer
   in `solved`. Each then checks the key, the wrong choices, the
   explanation and the topic tag.
2. **One fact-checker.** A third fresh subagent opens every source and
   checks every factual claim in the passage, question, answer and
   explanation.

A passage needs one solver-style pass and one fact-check pass. A question
on a passage waits until its passage is approved.

`cargo content-template <skill>` (or `stimuli-<subject>`) prints the
entries still needed, with `reviewer` and `role`. Each reviewer fills in
their own name. `cargo content-approve` sets an item reviewed only when
every role is covered and no reviewer has a current `fix` or `reject`.
Details are in `content/reviews/CHECKLIST.md`.
