# Writing cards and figures

Cards are the short **Quick review** for each principle: a cheat sheet for
someone who has already done the lesson. Full lessons follow
`docs/LESSON_STYLE.md` instead. Learners read them on a phone
in the middle of practice, so **every word has to earn its place**. Aim for a
high-school reading level: short sentences, everyday words, no jargon unless
the test uses it (then define it in a few words).

One file per skill: `content/cards/<skill>.toml`, one `[[card]]` per
principle of that skill (see `content/principles.toml` and the skill's
`targets` in `content/skills.toml`).

## Card format

```toml
[[card]]
id = "Q.3.d"                 # principle code
status = "draft"             # always draft; only `cargo content-approve` sets reviewed
title = "Percent problems"   # 1–5 words
key = '$$\text{part} = \text{percent} \times \text{whole}$$'
steps = [
  'Change the percent to a decimal: $15\% = 0.15$',
  'Multiply by the whole',
  'Discount: subtract it. Tax or tip: add it.',
]
example = '''
15% off \$60:
$$0.15 \times 60 = 9 \qquad 60 - 9 = 51$$
Sale price: **\$51**
'''
hack = '“Is over of”: $\frac{\text{is}}{\text{of}} = \frac{\%}{100}$. The number after “of” is the whole.'
```

Rules (the validator enforces most of these):

- **key**: the one rule or formula to remember. Use `$$...$$` for a display
  equation. At most a short phrase of words around it.
- **steps**: 1–4 steps, each at most 16 words (math doesn't count).
- **example**: one worked example, as few lines as possible. Show the work in
  display math (`$$...$$`). Bold the final answer with `**...**`.
- **hack**: a memory device or test shortcut learners can carry into the
  test (mnemonic, rhyme, picture, quick check, elimination trick). Make it
  memorable and correct.
- Whole card: **at most 90 words**, not counting anything inside `$...$`.
- Math goes in `$...$` (inline) or `$$...$$` (display). A literal dollar sign
  is `\$`. Use TOML literal strings (`'...'`, `'''...'''`) so backslashes need
  no escaping. A literal string can't contain `'''`; a single-line one can't
  contain `'`.
- Use **different numbers** from the skill's questions in `content/math/` so a
  card never gives away a question's answer.
- Metric units, US spelling. Original wording — never copy the GED guide.
- Optional `[card.figure]` (see below) when a picture teaches faster than
  words: graphs, number lines, shapes, data displays.

## Figures

A figure is a sub-table after the card or question it belongs to:

```toml
[question.figure]          # or [card.figure]
kind = "plane"
x_min = -2
x_max = 6
y_min = -4
y_max = 6
curves = ["2*x - 3"]       # expressions in x; use * for multiply, ^ for powers
points = [{ x = 0, y = -3, label = "(0, −3)" }, { x = 2, y = 1 }]
segments = [{ from = [0, -3], to = [1, -3], dashed = true, label = "run 1" }]
x_label = "hours"
y_label = "km"
```

Other kinds:

```toml
kind = "number_line"
min = -6
max = 6
points = [{ x = -2, open = false, label = "−2" }]
ray = { from = -2, dir = "right", open = false }    # x ≥ −2

kind = "bars"
bars = [{ label = "Mon", value = 1250 }, { label = "Tue", value = 980 }]
y_label = "salmon"

kind = "dots"
counts = [{ x = 0, count = 1 }, { x = 1, count = 3 }]
label = "fish caught"

kind = "box"
min = 52
q1 = 64
median = 71
q3 = 80
max = 95
axis_min = 50
axis_max = 100
label = "test score"

kind = "shape"   # geometry, no axes; coordinates only set the drawing
polygons = [{ points = [{ x = 0, y = 0 }, { x = 8, y = 0 }, { x = 8, y = 5 }, { x = 0, y = 5 }], side_labels = ["8 m", "5 m", "", ""], right_angles = [0] }]
circles = [{ x = 0, y = 0, r = 3, radius_label = "3 cm" }]
segments = [{ from = [4, 0], to = [4, 5], dashed = true, label = "h = 5 m" }]
labels = [{ x = 4, y = 2, label = "A = ?" }]
```

A figure on a question must match the question exactly (same numbers) and
must not reveal the answer. If you add a figure to an existing question, set
its `status = "draft"` (it needs a fresh review) and change nothing else.

Check your work with `cargo content` (0 errors in your files) and look at a
question with `cargo content-quiz <skill>`.
