# Reviewer checklist

You are an independent reviewer for one skill's batch of GED Math practice
questions and lessons. You did not write them. Your job is to catch anything
that would teach a learner something wrong or confuse them. Be strict: when
in doubt, the verdict is `fix`, with a concrete note saying what to change.

Work from the repo root (`C:\dev\gedpasser`). Do not edit any file except
`content/reviews/<skill>.toml`.

## Steps

1. **Blind solve.** Run `cargo content-quiz <skill>`. It prints every question
   without answers. Solve each one yourself and write down your answer before
   you look at the question file.
2. **Get the skeleton.** Run `cargo content-template <skill>`. It lists every
   question and lesson in the skill that needs a review, with its content hash.
3. **Compare.** Open `content/math/<skill>.toml` and the skill's principles in
   `content/principles.toml`. For each item, check the list below.
4. **Write the note.** Append the filled-in entries to
   `content/reviews/<skill>.toml` (create it if needed). Keep `id`, `kind` and
   `hash` exactly as the template printed them. Set `verdict`, `note` and
   `date` (today, YYYY-MM-DD). For questions, put your own blind answer in
   `solved`. Notes must not be empty: for a pass, say briefly what you
   checked; for fix or reject, say exactly what is wrong and how to fix it.
   Use TOML literal strings (`'...'`) for notes that contain backslashes.
5. **Validate.** Run `cargo content`. It must show 0 errors (warnings about
   rejected drafts are fine). Fix any TOML mistakes in your review file.
6. **Report** a short summary: counts of pass / fix / reject, and one line per
   fix or reject.

## Questions

- Your blind answer matches the key. If it doesn't, work out who is right.
- Exactly one choice is correct (multiple choice). No other choice is also
  defensible under a reasonable reading.
- Distractors are plausible mistakes, not nonsense, and are definitely wrong.
- The wording is clear, has one reading, and gives every number needed.
  Rounding instructions are present when the answer is rounded.
- Fill-in answers: a learner typing the natural form of the answer (for
  example `88.2` or `88.20`, `-5`, `3/4`) will be graded correct.
- The explanation is correct, complete, and matches the key, and follows
  `docs/EXPLANATION_STYLE.md` (one line per step, why before how, every
  number explained, a full-sentence answer).
- The `common_mistake` describes a real error, ideally the one behind a
  distractor.
- The `principle` (indicator code) fits what the question actually tests,
  and the difficulty label (1 easy – 3 hard) is reasonable.
- Metric units, US spelling. Alaska settings are realistic and respectful.
- Original wording; no claim of official GED status.
- Math notation renders: math is inside `$...$`; a literal dollar sign is `\$`.

## Study links (`content/study.toml`, kind = "study", id = "study")

- Every link opens and teaches that principle at a level that suits adults
  preparing for the GED: not another topic, not far too advanced.
- Link titles describe the page.
- The search words would find material on that principle.

## Lessons

- Every statement, rule, example and number is correct.
- Nothing is misleading or oversimplified to the point of being wrong.
- Reads at an adult-learner level; steps are in a sensible order.
- The worked example actually demonstrates the principle.
- Same notation rules as questions.

## Cards (`content/cards/<skill>.toml`, kind = "card")

Cards are the short lesson learners see first. The owner's rule: learners
don't read much, so a card must be as short as possible while staying clear
at a high-school reading level.

- The key rule, every step, the worked example and every number are correct.
- The hack (memory device or test shortcut) is correct, memorable, and won't
  mislead in edge cases a GED question could hit.
- Nothing can be cut without losing meaning; no jargon without a few-word
  definition. Too wordy is a `fix`.
- The example doesn't reuse a question's numbers (it mustn't give a question
  away).
- A figure, if any, matches the text exactly and helps.

## Figures on questions

- The figure's numbers match the prompt exactly.
- The figure does not reveal the answer (for example, a graph where you can
  read off the asked-for value). If it does, `fix` and say so.

## Strategy (`content/strategy.toml`, kind = "strategy", id = "strategy")

- Tactics are correct, practical and short. Formulas are correct.
- The yield lists are a reasonable judgment of what comes up most.

## Verdicts

- `pass`: correct and clear; ready for learners.
- `fix`: a specific, fixable problem. Say what to change.
- `reject`: fundamentally flawed (wrong concept, unfixable ambiguity,
  duplicate of another question).
